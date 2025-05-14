//! Implementation of [`TaskContext`]
use crate::trap::trap_return;
use super::coroutine::CoroutineContext;
#[repr(C)]
/// task context structure containing some registers
pub struct TaskContext {
    /// return address ( e.g. __restore ) of __switch ASM function
    ra: usize,
    /// kernel stack pointer of app
    sp: usize,
    /// s0-11 register, callee saved
    s: [usize; 12],
}
unsafe extern "C" {
    pub fn __switch_co(
        current_ctx: *mut CoroutineContext,
        next_ctx: *const CoroutineContext,
    );
}pub unsafe fn coroutine_switch(
    current_ctx: *mut CoroutineContext,
    next_ctx: *const CoroutineContext,
) {
    __switch_co(current_ctx, next_ctx);
}
impl TaskContext {
    /// init task context
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }
    /// set Task Context{__restore ASM funciton: trap_return, sp: kstack_ptr, s: s_0..12}
    pub fn goto_trap_return(kstack_ptr: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp: kstack_ptr,
            s: [0; 12],
        }
    }
}
