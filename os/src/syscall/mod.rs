const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_MMAP: usize = 222;
const SYSCALL_MUNMAP: usize = 215;

mod fs;
mod process;

use fs::*;
use process::*;

pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(args[0] as *mut TimeVal, args[1]),
        SYSCALL_MMAP => sys_mmap(args[0], args[1], args[2]),
        SYSCALL_MUNMAP => sys_munmap(args[0], args[1]),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}

///在引入虚拟内存之前，内核可以直接访问物理地址。sys_get_time 可能直接操作某个硬件计时器寄存器并将结果返回给用户。
/// 但引入虚拟内存后，内核和用户空间有了隔离。用户传递给内核的指针（比如用于接收 TimeVal 结构体的指针）是虚拟地址，
/// 内核不能直接解引用这个用户空间的虚拟地址。内核需要先将用户传入的虚拟地址指针转换为内核可以访问的物理地址或映射
/// 到内核空间的地址，然后才能写入时间数据。