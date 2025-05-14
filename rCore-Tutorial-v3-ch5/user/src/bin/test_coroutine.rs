// user/src/bin/coroutine_test.rs
#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{coroutine_create, coroutine_yield, coroutine_resume, CoroutineId};

// 协程1的执行函数
fn coroutine1(arg: usize) -> i32 {
    println!("Coroutine 1 started with arg: {}", arg);
    for i in 0..3 {
        println!("Coroutine 1: {}", i);
        coroutine_yield();
    }
    println!("Coroutine 1 finished");
    0
}

// 协程2的执行函数
fn coroutine2(arg: usize) -> i32 {
    println!("Coroutine 2 started with arg: {}", arg);
    for i in 0..5 {
        println!("Coroutine 2: {}", i);
        coroutine_yield();
    }
    println!("Coroutine 2 finished");
    0
}

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("Coroutine test started");

    // 创建两个协程
    let cid1 = coroutine_create(coroutine1, 1);
    println!("Created coroutine 1 with id: {}", cid1);

    let cid2 = coroutine_create(coroutine2, 2);
    println!("Created coroutine 2 with id: {}", cid2);

    // 交替恢复两个协程的执行
    for _ in 0..10 {
        coroutine_resume(cid1);
        coroutine_resume(cid2);
    }

    println!("Coroutine test finished");
    0
}