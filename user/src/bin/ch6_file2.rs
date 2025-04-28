//核心测试，验证 sys_linkat, sys_unlinkat, sys_fstat 的交互。
#![no_std]
#![no_main]
#![reexport_test_harness_main = "test_main"]
#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]

#[macro_use]
extern crate user_lib;
use user_lib::{close, fstat, link, open, read, unlink, write, OpenFlags, Stat};

/// 测试 link/unlink，输出　Test link OK! 就算正确。

#[no_mangle]
pub fn main() -> i32 {
    let test_str = "Hello, world!";
    let fname = "fname2\0";
    let (lname0, lname1, lname2) = ("linkname0\0", "linkname1\0", "linkname2\0");

    // 创建文件fname2.此时fname2的inode的nlink是1
    let fd = open(fname, OpenFlags::CREATE | OpenFlags::WRONLY) as usize; 

    // 调用sys_linkat
    link(fname, lname0);
    let stat = Stat::new();

    // 调用sys_fstat,sys_fstat会获取文件描述符fd对应的VFS Inode，然后调用类似 vfs::Inode::get_link_num 的方法（或者直接读取 inode 元数据，如果 nlink 存储在 inode 中）来获取链接数。
    fstat(fd, &stat);
    assert_eq!(stat.nlink, 2);

    // 创建另外两个硬链接.都指向fname2的inode
    link(fname, lname1);
    link(fname, lname2);

    // 再次获取链接数
    fstat(fd, &stat);

    // 断言链接数是4
    assert_eq!(stat.nlink, 4);
    write(fd, test_str.as_bytes());
    close(fd);

    unlink(fname);
    let fd = open(lname0, OpenFlags::RDONLY) as usize;
    let stat2 = Stat::new();
    let mut buf = [0u8; 100];
    let read_len = read(fd, &mut buf) as usize;
    assert_eq!(test_str, core::str::from_utf8(&buf[..read_len]).unwrap(),);
    fstat(fd, &stat2);
    assert_eq!(stat2.dev, stat.dev);
    assert_eq!(stat2.ino, stat.ino);
    assert_eq!(stat2.nlink, 3);

    // 下面测试unlink
    unlink(lname1);
    unlink(lname2);
    fstat(fd, &stat2);
    assert_eq!(stat2.nlink, 1);
    close(fd);
    unlink(lname0);
    // It's Ok if you don't delete the inode and data blocks.
    println!("Test link OK!");
    0
}

pub fn test_runner(_test: &[&dyn Fn()]) {
    loop {}
}
