//内核启动的第一个用户进程 (PID 1)。它可以是 shell，也可以是 usertest 运行器。负责初始化用户环境和管理进程。
#![no_std]
#![no_main]
#![reexport_test_harness_main = "test_main"]
#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]


#[macro_use]
extern crate user_lib;

use user_lib::{exec, fork, wait, yield_};

#[no_mangle]
fn main() -> i32 {
    if fork() == 0 {
        exec("ch6b_user_shell\0", &[0 as *const u8]);
    } else {
        loop {
            let mut exit_code: i32 = 0;
            let pid = wait(&mut exit_code);
            if pid == -1 {
                yield_();
                continue;
            }
            println!(
                "[initproc] Released a zombie process, pid={}, exit_code={}",
                pid, exit_code,
            );
        }
    }
    0
}

pub fn test_runner(_test: &[&dyn Fn()]) {
    loop {}
}