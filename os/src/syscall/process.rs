use crate::task::{
    suspend_current_and_run_next,
    exit_current_and_run_next,
    current_task,
    current_user_token,
    add_task,
};
use crate::mm::{
    translated_str,
    translated_refmut,
};
use crate::loader::get_app_data_by_name;
use alloc::sync::Arc;
use crate::timer::get_time_us;
use crate::task::task::TaskControlBlock;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn sys_exit(exit_code: i32) -> ! {
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let task = current_task().unwrap();
    // find a child process

    // ---- access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    if inner.children
        .iter()
        .find(|p| {pid == -1 || pid as usize == p.getpid()})
        .is_none() {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children
        .iter()
        .enumerate()
        .find(|(_, p)| {
            // ++++ temporarily access child PCB lock exclusively
            p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
            // ++++ release child PCB
        });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after removing from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child TCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB lock automatically
}

pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let us = get_time_us();
    let token = current_user_token();
    let ts_kernel = translated_refmut(token, ts);
    *ts_kernel = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    0
}

/// sys_spawn 的目标是创建一个全新的子进程，并让这个子进程直接开始执行指定的程序（由 path 参数指定）。这与传统的 fork + exec 不同：
/// fork: 复制父进程的几乎所有状态（包括内存空间）。
/// exec: 用新的程序镜像替换当前进程的内存空间和执行状态。
/// spawn: 一步到位，创建一个新的、独立的进程，并加载指定程序，不复制父进程的内存空间。
pub fn sys_spawn(path: *const u8) -> isize {
    // 获取当前用户token
    let token = current_user_token();
    //从用户传入的指针 path 获取程序路径字符串：
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        // 直接创建一个新的任务控制块
        let new_task = Arc::new(TaskControlBlock::new(data));
        let pid = new_task.getpid();
        // 将当前进程设为新进程的父进程
        let current_task = current_task().unwrap();
        let mut current_inner = current_task.inner_exclusive_access();
        current_inner.children.push(new_task.clone());
        // 添加新任务到调度器
        add_task(new_task);
        pid as isize
    } else {
        -1
    }
}

// mmap和munmap系统调用
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    
    // 检查参数是否合法
    // 映射起始地址必须是页对齐的
    if start % 4096 != 0 {
        return -1;
    }
    // 映射长度必须大于0
    if len <= 0 {
        return -1;
    }
    
    // 添加映射
    let result = inner.memory_set.mmap(start, len, prot);
    if result.is_ok() {
        0
    } else {
        -1
    }
}

pub fn sys_munmap(start: usize, len: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    
    // 检查参数是否合法
    if start % 4096 != 0 {
        return -1;
    }
    if len <= 0 {
        return -1;
    }
    
    // 取消映射
    let result = inner.memory_set.munmap(start, len);
    if result.is_ok() {
        0
    } else {
        -1
    }
}