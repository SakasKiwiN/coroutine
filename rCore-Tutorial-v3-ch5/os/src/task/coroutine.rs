// os/src/task/coroutine.rs
use crate::sync::UPSafeCell;
use alloc::sync::{Arc};
use alloc::vec::Vec;
use alloc::collections::VecDeque;
use core::cell::RefMut;

// 协程的状态
#[derive(Copy, Clone, PartialEq)]
pub enum CoroutineStatus {
    Ready,
    Running,
    Blocked,
    Exited,
}

// 协程的上下文
pub struct CoroutineContext {
    // 通用寄存器的值
    pub ra: usize,
    pub sp: usize,
    pub s: [usize; 12], // s0-s11 寄存器
}

// 协程控制块
pub struct CoroutineControlBlock {
    // 协程ID
    pub cid: usize,
    // 协程内部数据
    inner: UPSafeCell<CoroutineInner>,
}

pub struct CoroutineInner {
    // 协程状态
    pub status: CoroutineStatus,
    // 协程上下文
    pub context: CoroutineContext,
    // 协程栈的虚拟地址空间起始地址
    pub stack_base: usize,
    // 协程栈大小
    pub stack_size: usize,
    // 协程入口点函数
    pub entry: usize,
    // 协程函数参数
    pub arg: usize,
}

impl CoroutineControlBlock {
    // 创建新协程
    pub fn new(entry: usize, arg: usize, stack_size: usize, stack_base: usize) -> Self {
        // 为协程分配一个唯一的ID
        static mut NEXT_CID: usize = 0;
        let cid = unsafe {
            NEXT_CID += 1;
            NEXT_CID
        };

        Self {
            cid,
            inner: unsafe {
                UPSafeCell::new(CoroutineInner {
                    status: CoroutineStatus::Ready,
                    context: CoroutineContext {
                        ra: entry,
                        sp: stack_base + stack_size, // 栈从高地址向低地址增长
                        s: [0; 12],
                    },
                    stack_base,
                    stack_size,
                    entry,
                    arg,
                })
            },
        }
    }

    // 获取可变引用
    pub fn inner_exclusive_access(&self) -> RefMut<'_, CoroutineInner> {
        self.inner.exclusive_access()
    }
}

// 协程管理器 - 每个任务有一个管理器来管理其协程
pub struct CoroutineManager {
    // 所有协程列表
    coroutines: Vec<Arc<CoroutineControlBlock>>,
    // 当前运行的协程ID
    current_coroutine: Option<usize>,
    // 就绪队列
    ready_queue: VecDeque<Arc<CoroutineControlBlock>>,
    // 阻塞队列
    blocked_queue: Vec<Arc<CoroutineControlBlock>>,
    // 下一个可用的栈基址
    next_stack_base: usize,
}

impl CoroutineManager {
    pub fn new() -> Self {
        Self {
            coroutines: Vec::new(),
            current_coroutine: None,
            ready_queue: VecDeque::new(),
            blocked_queue: Vec::new(),
            next_stack_base: 0x8000_0000, // 从用户空间的某个区域开始分配栈空间
        }
    }

    // 创建新协程
    pub fn create_coroutine(&mut self, entry: usize, arg: usize, stack_size: usize) -> Arc<CoroutineControlBlock> {
        let stack_base = self.next_stack_base;
        // 更新下一个可用栈基址（避免栈空间重叠）
        self.next_stack_base += stack_size;

        let coroutine = Arc::new(CoroutineControlBlock::new(entry, arg, stack_size, stack_base));

        // 将协程添加到列表和就绪队列
        self.coroutines.push(coroutine.clone());
        self.ready_queue.push_back(coroutine.clone());

        coroutine
    }

    // 切换到下一个就绪的协程
    pub fn switch_to_next_coroutine(&mut self) -> Option<(Arc<CoroutineControlBlock>, Arc<CoroutineControlBlock>)> {
        // 如果没有就绪的协程，返回None
        if self.ready_queue.is_empty() {
            return None;
        }

        // 获取当前运行的协程
        let current = if let Some(cid) = self.current_coroutine {
            // 找到当前协程
            let mut current_coroutine = None;
            for coroutine in &self.coroutines {
                if coroutine.cid == cid {
                    current_coroutine = Some(coroutine.clone());
                    break;
                }
            }

            // 如果找到了当前协程，将其状态设为就绪并加入就绪队列
            if let Some(coroutine) = &current_coroutine {
                let mut inner = coroutine.inner_exclusive_access();
                if inner.status == CoroutineStatus::Running {
                    inner.status = CoroutineStatus::Ready;
                    drop(inner);
                    self.ready_queue.push_back(coroutine.clone());
                }
            }

            current_coroutine
        } else {
            None
        };

        // 从就绪队列取出下一个协程
        if let Some(next_coroutine) = self.ready_queue.pop_front() {
            let cid = next_coroutine.cid;
            let mut inner = next_coroutine.inner_exclusive_access();
            inner.status = CoroutineStatus::Running;
            drop(inner);

            self.current_coroutine = Some(cid);

            if let Some(current) = current {
                Some((current, next_coroutine))
            } else {
                // 如果没有当前协程，返回None作为当前协程
                None
            }
        } else {
            None
        }
    }

    // 将当前协程设置为阻塞状态
    pub fn block_current_coroutine(&mut self) {
        if let Some(cid) = self.current_coroutine {
            for coroutine in &self.coroutines {
                if coroutine.cid == cid {
                    let mut inner = coroutine.inner_exclusive_access();
                    inner.status = CoroutineStatus::Blocked;
                    drop(inner);

                    self.blocked_queue.push(coroutine.clone());
                    self.current_coroutine = None;
                    break;
                }
            }
        }
    }

    // 唤醒一个阻塞的协程
    pub fn unblock_coroutine(&mut self, cid: usize) -> bool {
        let mut found_index = None;
        for (i, coroutine) in self.blocked_queue.iter().enumerate() {
            if coroutine.cid == cid {
                let mut inner = coroutine.inner_exclusive_access();
                inner.status = CoroutineStatus::Ready;
                drop(inner);

                found_index = Some(i);
                break;
            }
        }

        if let Some(index) = found_index {
            let coroutine = self.blocked_queue.remove(index);
            self.ready_queue.push_back(coroutine);
            true
        } else {
            false
        }
    }
}