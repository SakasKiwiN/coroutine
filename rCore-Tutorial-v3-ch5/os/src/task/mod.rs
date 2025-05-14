//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the whole operating system.
//!
//! A single global instance of [`Processor`] called `PROCESSOR` monitors running
//! task(s) for each core.
//!
//! A single global instance of [`PidAllocator`] called `PID_ALLOCATOR` allocates
//! pid for user apps.
//!
//! Be careful when you see `__switch __switch_co` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.
mod context;
mod manager;
mod pid;
mod processor;
mod switch;
#[allow(clippy::module_inception)]
mod task;
mod coroutine;
use crate::sync::UPSafeCell;
use crate::loader::get_app_data_by_name;
use crate::sbi::shutdown;
use alloc::sync::Arc;
use lazy_static::*;
pub use manager::{TaskManager, fetch_task};
use switch::__switch;



use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;
pub use manager::add_task;
pub use pid::{KernelStack, PidAllocator, PidHandle, pid_alloc};

use crate::task::context::coroutine_switch;
pub use processor::{
    Processor, current_task, current_trap_cx, current_user_token, run_tasks, schedule,
    take_current_task,
};
pub use coroutine::{
    CoroutineControlBlock, CoroutineContext, CoroutineStatus, CoroutineManager
};
/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_task().unwrap();

    // ---- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // ---- release current PCB

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr);
}

/// pid of usertests app in make run TEST=1
pub const IDLE_PID: usize = 0;

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();

    let pid = task.getpid();
    if pid == IDLE_PID {
        println!(
            "[kernel] Idle process exit with exit_code {} ...",
            exit_code
        );
        if exit_code != 0 {
            //crate::sbi::shutdown(255); //255 == -1 for err hint
            shutdown(true)
        } else {
            //crate::sbi::shutdown(0); //0 for success hint
            shutdown(false)
        }
    }

    // **** access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    // Change status to Zombie
    inner.task_status = TaskStatus::Zombie;
    // Record exit code
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    // ++++++ access initproc TCB exclusively
    {
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    // ++++++ release parent PCB

    inner.children.clear();
    // deallocate user space
    inner.memory_set.recycle_data_pages();
    drop(inner);
    // **** release current PCB
    // drop task manually to maintain rc correctly
    drop(task);
    // we do not have to save task context
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

lazy_static! {
    ///Globle process that init user shell
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new(TaskControlBlock::new(
        get_app_data_by_name("initproc").unwrap()
    ));
    /// 全局协程管理器，负责协程的创建、调度和状态管理
    pub static ref COROUTINE_MANAGER: UPSafeCell<CoroutineManager> = unsafe {
        UPSafeCell::new(CoroutineManager::new())
    };
}
///Add init process to the manager
pub fn add_initproc() {
    add_task(INITPROC.clone());
}

/// Create coroutine
pub fn coroutine_create(entry: usize, arg: usize, stack_size: usize) -> Arc<CoroutineControlBlock> {
    COROUTINE_MANAGER.exclusive_access().create_coroutine(entry, arg, stack_size)
}

/// Yield current coroutine
pub fn coroutine_yield() -> bool {
    if let Some((current, next)) = COROUTINE_MANAGER.exclusive_access().switch_to_next_coroutine() {
        // 获取两个协程的上下文指针
        let current_ctx_ptr = &mut current.inner_exclusive_access().context as *mut CoroutineContext;
        let next_ctx_ptr = &next.inner_exclusive_access().context as *const CoroutineContext;

        unsafe {
            coroutine_switch(current_ctx_ptr, next_ctx_ptr);
        }
        true
    } else {
        false
    }
}

/// Block current coroutine
pub fn coroutine_block() {
    COROUTINE_MANAGER.exclusive_access().block_current_coroutine();
}

/// Resume a coroutine
pub fn coroutine_resume(cid: usize) -> bool {
    let mut manager = COROUTINE_MANAGER.exclusive_access();
    if manager.unblock_coroutine(cid) {
        // 如果成功唤醒，尝试切换
        drop(manager);
        coroutine_yield()
    } else {
        false
    }
}
