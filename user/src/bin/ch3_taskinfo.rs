#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{println, task_info, write, get_time, MAX_SYSCALL_NUM};
// 引入一些关键函数和常量：
// println!：用于打印信息
// task_info：获取任务信息
// write：写入标准输出
// get_time：获取当前时间
// MAX_SYSCALL_NUM：系统调用数量


#[no_mangle] //使用#[no_mangle]注解，确保函数名不变
pub fn main() -> i32 {
    // 先执行一些系统调用，让系统调用计数器增加
    println!("Hello TaskInfo!");
    
    // 执行一些其他系统调用，向stdout（文件描述符1）写入3次数据，这会增加系统调用计数器
    for _ in 0..3 {
        write(1, "Writing to stdout\n".as_bytes());
    }
    
    // 获取当前任务的信息
    let id = 0; // 当前任务的ID
    if let Some(info) = task_info(id) { //task_info函数获取当前任务的信息
        // 打印任务状态信息，包括状态（Running/Ready/Waiting）和运行时间
        println!("Task {} info:", id);
        println!("Status: {:?}", info.status);
        println!("Time: {} ms", info.time);
        // 打印系统调用计数器信息，包括每个系统调用的ID和次数
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
    // 获取当前时间
    let start_time = get_time();
    // 设置睡眠时间
    let sleep_ms = 100;
    println!("Sleeping for {} ms...", sleep_ms);
    // 睡眠一段时间
    user_lib::sleep(sleep_ms);
    // 计算睡眠时间
    let elapsed = get_time() - start_time;
    println!("Elapsed time: {} ms", elapsed);
    
    // 再次获取任务信息,睡眠结束后再次查看任务时间，验证运行时间更新
    if let Some(info) = task_info(id) {
        println!("Updated task time: {} ms", info.time);
    }
    
    0
} 

// 这是一个测试程序，测试和验证操作系统的系统调用功能是否正确工作，尤其是：
// 1. 多个系统调用能否正常进行
// 2. 系统调用计数器是否正确更新
// 3. 任务状态信息是否正确
// 4. 任务运行时间是否正确
// 5. 睡眠功能是否正常
// 6. 任务信息获取功能是否正常

// 程序的执行流程：
// 1. 用户程序编译为ELF二进制。这段代码会通过编译器编译成一个用户态可执行文件（比如 ELF 格式），操作系统内核负责加载这个程序并开始运行。
// 2. 启动时由内核加载并调度运行。操作系统在某个任务 slot（比如 PID = 0）里创建一个任务，加载这个用户程序。设置好栈、程序入口地址后开始调度运行它。
// 3. 执行到系统调用（write、sleep 等）。比如 write(fd, buf) 不是 Rust 普通函数，而是通过 ecall 指令进入内核。内核通过 syscall handler 匹配 syscall ID，执行对应功能，然后返回。
// 4. task_info(id) 实际上是一次“查询当前任务”系统调用。内核会将当前任务的信息（状态、时间、系统调用次数）封装成结构体返回。
// 5. 最终程序退出，或返回值交由内核处理（exit）

