#![no_std]
#![no_main]

extern crate user_lib;

use user_lib::{println, task_info, write, get_time, MAX_SYSCALL_NUM};

#[no_mangle]
pub fn main() -> i32 {
    // 先执行一些系统调用，让系统调用计数器增加
    println!("Hello TaskInfo!");
    
    // 执行一些其他系统调用
    for _ in 0..3 {
        write(1, "Writing to stdout\n".as_bytes());
    }
    
    // 获取当前任务的信息
    let id = 0; // 当前任务的ID
    if let Some(info) = task_info(id) {
        println!("Task {} info:", id);
        println!("Status: {:?}", info.status);
        println!("Time: {} ms", info.time);
        
        println!("System calls:");
        let mut total_syscalls = 0;
        for i in 0..MAX_SYSCALL_NUM {
            if info.call[i].times > 0 {
                println!("  ID: {}, Times: {}", info.call[i].id, info.call[i].times);
                total_syscalls += info.call[i].times;
            }
        }
        println!("Total system calls: {}", total_syscalls);
    } else {
        println!("Failed to get task info!");
    }
    
    // 测试睡眠并再次检查任务信息
    let start_time = get_time();
    let sleep_ms = 100;
    println!("Sleeping for {} ms...", sleep_ms);
    user_lib::sleep(sleep_ms);
    let elapsed = get_time() - start_time;
    println!("Elapsed time: {} ms", elapsed);
    
    // 再次获取任务信息
    if let Some(info) = task_info(id) {
        println!("Updated task time: {} ms", info.time);
    }
    
    0
} 