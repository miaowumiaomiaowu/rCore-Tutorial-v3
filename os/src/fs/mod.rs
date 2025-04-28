//! File trait & inode(dir, file, pipe, stdin, stdout)

mod inode;
mod stdio;

use crate::mm::UserBuffer;
use core::any::Any;

/// convert current type to &dyn Any
pub trait AnyConvertor {
    fn as_any(&self) -> &dyn Any;
}

impl<T: 'static> AnyConvertor for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// 定义文件对象的基本操作
pub trait File: Send + Sync + AnyConvertor {
    /// the file readable?
    fn readable(&self) -> bool;
    /// the file writable?
    fn writable(&self) -> bool;
    /// read from the file to buf, return the number of bytes read
    fn read(&self, buf: UserBuffer) -> usize;
    /// write to the file from buf, return the number of bytes written
    fn write(&self, buf: UserBuffer) -> usize;
}

/// The stat of a inode
#[repr(C)]
#[derive(Debug)]
pub struct Stat {
    pub dev: u64,//设备号（硬编码为0）
    pub ino: u64,//inode编号
    pub mode: StatMode,//文件类型和模式
    pub nlink: u32,//硬链接数
    pub pad: [u64; 7],// 填充以匹配 C 结构体大小
}

// 定义文件模式 (类型)
bitflags! {
    /// The mode of a inode
    /// whether a directory or a file
    pub struct StatMode: u32 {
        /// null
        const NULL  = 0;
        /// directory
        const DIR   = 0o040000;//目录
        /// ordinary regular file
        const FILE  = 0o100000;//普通文件
    }
}

pub use inode::{list_apps, open_file, OSInode, OpenFlags, ROOT_INODE};
pub use stdio::{Stdin, Stdout};
