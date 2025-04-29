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
    let us = get_time_us();//获取当前时间，获取微秒
    let token = current_user_token();// 获取任务的页表
    let mut buffers = translated_byte_buffer(token, ts as *const u8, core::mem::size_of::<TimeVal>());
    /// 这个函数接收当前任务的页表token，和用户传入的虚拟地址指针ts，和期望写入的长度(TimeVal的大小）
    /// 它会查找页表，将用户虚拟地址转换为内核可以直接访问的物理地址
    /// 或者更准确地说，是内核映射的一段缓冲区，其内容对应用户虚拟地址指向的物理内存
    if buffers.len() == 0 {
        return -1;
    }
    ///将获取到的时间（秒和微秒）写入到这个内核可以直接访问的缓冲区中。
    /// 由于这段缓冲区映射到了用户空间指针指向的物理内存，用户空间后续就能读到更新后的值。

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

    // 将长度按页向上取整,取整到PAGE_SIZE的整数倍
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
    //传入转换后的虚拟地址范围和内核权限
}

pub fn sys_munmap(start: usize, len: usize) -> isize {
    // 检查参数合法性
    // 检查 start 地址是否按 PAGE_SIZE 对齐。
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    //如果 len 为 0，直接返回 0。
    if len == 0 {
        return 0;
    }
    
    // 长度处理: 将 len 向上取整到 PAGE_SIZE 的整数倍。
    let length = if len % PAGE_SIZE == 0 {
        len
    } else {
        (len / PAGE_SIZE + 1) * PAGE_SIZE
    };

    // 使调用辅助函数: 将处理后的起始/结束虚拟地址 (VirtAddr) 传递给 task_munmap 函数。
    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(start + length);
    
    task_munmap(start_va, end_va)
}