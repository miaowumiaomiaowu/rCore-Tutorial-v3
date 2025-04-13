use crate::task::{
    suspend_current_and_run_next,
    exit_current_and_run_next,
    task_mmap, task_munmap
};
use crate::timer::get_time_us;
use crate::mm::translated_byte_buffer;
use crate::mm::{MapPermission, VirtAddr};
use crate::task::current_user_token;
use crate::config::PAGE_SIZE;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn sys_exit(exit_code: i32) -> ! {
    println!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let us = get_time_us();
    let token = current_user_token();
    let mut buffers = translated_byte_buffer(token, ts as *const u8, core::mem::size_of::<TimeVal>());
    if buffers.len() == 0 {
        return -1;
    }
    unsafe {
        let time_val = buffers[0].as_mut_ptr() as *mut TimeVal;
        *time_val = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    // 检查参数合法性
    // 1. start 必须按页对齐
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    // 2. prot有效位 (0-2) 以外的位必须为0
    if prot & !0x7 != 0 {
        return -1;
    }
    // 3. prot 至少有一个有效位是1，否则无意义
    if prot & 0x7 == 0 {
        return -1;
    }
    // 4. 长度为0，直接返回成功
    if len == 0 {
        return 0;
    }

    // 将长度按页向上取整
    let length = if len % PAGE_SIZE == 0 {
        len
    } else {
        (len / PAGE_SIZE + 1) * PAGE_SIZE
    };

    // 构建映射权限
    let mut permission = MapPermission::U;
    if (prot & 0x1) != 0 { permission |= MapPermission::R; }
    if (prot & 0x2) != 0 { permission |= MapPermission::W; }
    if (prot & 0x4) != 0 { permission |= MapPermission::X; }

    // 获取当前任务的内存集并插入新的帧区域
    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(start + length);
    
    // 通过辅助函数完成mmap
    task_mmap(start_va, end_va, permission)
}

pub fn sys_munmap(start: usize, len: usize) -> isize {
    // 检查参数合法性
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    
    // 将长度按页向上取整
    let length = if len % PAGE_SIZE == 0 {
        len
    } else {
        (len / PAGE_SIZE + 1) * PAGE_SIZE
    };

    // 使用辅助函数完成munmap
    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(start + length);
    
    task_munmap(start_va, end_va)
}