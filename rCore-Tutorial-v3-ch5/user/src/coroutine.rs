// user/src/coroutine.rs
use crate::syscall::{syscall};

// 系统调用号
const SYSCALL_COROUTINE_CREATE: usize = 600;
const SYSCALL_COROUTINE_YIELD: usize = 601;
const SYSCALL_COROUTINE_RESUME: usize = 602;
const SYSCALL_COROUTINE_EXIT: usize = 603;

// 协程ID类型
pub type CoroutineId = usize;

// 协程函数类型
pub type CoroutineFunc = fn(usize) -> i32;

// 定义默认栈大小
const DEFAULT_STACK_SIZE: usize = 8192; // 8KB

// 协程创建包装函数
pub fn coroutine_create(func: CoroutineFunc, arg: usize) -> CoroutineId {
    let func_addr = func as usize;
    syscall(SYSCALL_COROUTINE_CREATE, [func_addr, arg, DEFAULT_STACK_SIZE]) as CoroutineId
}

// 协程主动让出CPU
pub fn coroutine_yield() -> isize {
    syscall(SYSCALL_COROUTINE_YIELD, [0, 0, 0])
}

// 恢复指定协程的执行
pub fn coroutine_resume(cid: CoroutineId) -> isize {
    syscall(SYSCALL_COROUTINE_RESUME, [cid, 0, 0])
}

// 退出当前协程
pub fn coroutine_exit(exit_code: i32) -> ! {
    syscall(SYSCALL_COROUTINE_EXIT, [exit_code as usize, 0, 0]);
    panic!("coroutine exit failed");
}