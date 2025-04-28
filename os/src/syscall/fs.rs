// 这里是文件相关的系统调用

use crate::mm::{translated_byte_buffer, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};
use crate::fs::{open_file, OpenFlags, Stat, ROOT_INODE, OSInode, StatMode};
use core::any::Any;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// YOUR JOB: Implement fstat.
/// 向下转型: 它依赖于 downcast_ref 将 Arc<dyn File> 转换为 &OSInode 来访问具体方法。
/// 动态 nlink: 明确调用了 ROOT_INODE.get_link_num() 来实时计算链接数。
/// 硬编码 mode: mode 被硬编码为 StatMode::FILE，这意味着即使用户对一个目录调用 fstat，返回的模式也会错误地显示为普通文件。正确的实现应该查询 OSInode 或底层 DiskInode 的类型。
/// 栈上构建 Stat: Stat 结构体是在内核栈上临时创建的，然后其内容被复制到用户空间。
pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    // 1. 获取当前任务和 TCB 内部访问权
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();

    // 2. 检查 fd 合法性
    if fd >= inner.fd_table.len() {
        return -1;//无效fd
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }

    // 3. 获取 inode 信息和链接数
    let ino: u64;
    let nlink: u32;
    if let Some(file_node) = &inner.fd_table[fd] {
        // 3.1. 将 File trait 对象向下转型为具体的 OSInode 类型
        //      这假设了文件描述符表中存储的总是 OSInode
        let any: &dyn Any = file_node.as_any();
        let os_node = any.downcast_ref::<OSInode>().unwrap();

        // 3.2. 获取 inode 编号
        ino = os_node.get_inode_id();

        // 3.3. 获取 inode 的物理位置 (块号, 块内偏移)
        let (block_id, block_offset) = os_node.get_inode_pos();

        // 3.4. 动态计算硬链接数
        //      调用根目录的 get_link_num 方法，传入 inode 的物理位置
        //      get_link_num 会扫描根目录，统计有多少目录项指向这个位置
        nlink = ROOT_INODE.get_link_num(block_id, block_offset);
    } else {
        // 理论上不会执行到这里，因为前面已经检查过 is_none()
        return -1;
    }

    // 4. 在内核栈上构建 Stat 结构体
    let stat = &Stat {
        dev: 0,// 设备号硬编码为 0
        ino: ino,// 使用获取到的 inode 编号
        mode: StatMode::FILE,// !! 注意：这里硬编码为 FILE，没有检查实际类型是否为 DIR
        nlink: nlink,// 使用动态计算出的链接数
        pad: [0;7],
    };

     // 5. 将内核 Stat 结构体复制到用户空间
    let token = inner.get_user_token();
    // 获取用户空间缓冲区的安全访问切片
    let st = translated_byte_buffer(token, st as *const u8, core::mem::size_of::<Stat>());
    // 将内核 stat 转为字节指针
    let stat_ptr = stat as *const _ as *const u8;
    // 遍历用户缓冲区切片并复制数据
    for (idx, byte) in st.into_iter().enumerate() {
        unsafe {
            byte.copy_from_slice(core::slice::from_raw_parts(stat_ptr.wrapping_byte_add(idx), byte.len()));
        }
    }
    0
}



// 添加sys_linkat函数，为oldpath创建一个名为newpath的硬链接
pub fn sys_linkat(old_name: *const u8, new_name: *const u8) -> isize {
    // 获取当前用户空间的token,用于后续安全访问用户内存
    let token = current_user_token();
    //  使用 token 将用户态指针 old_name（即 old_path_ptr）转换为内核可以安全访问的字符串 old。如果用户指针无效，这里会出错。
    let old = translated_str(token, old_name);
    // 同上，转换 new_path_ptr 为内核字符串 new。
    let new = translated_str(token, new_name);
    println!("link {} to {}", new , old);
    // 检查原始路径和新路径是否相同，如果相同返回-1
    if old.as_str() != new.as_str() {
        // 调用VFS层提供的link方法。
        // ROOT_INODE 是一个全局的、代表文件系统根目录的 Arc<Inode>（这里的 Inode 是 vfs.rs 中定义的 VFS Inode）。
        if let Some(_) = ROOT_INODE.link(old.as_str(), new.as_str()) {
            return 0;
        }
    }
    -1
}

/// YOUR JOB: Implement unlinkat.
/// sys_unlink：获取用户传入的路径，然后调用VFS层，ran（即 easy-fs 的 Inode）的 unlink 方法来执行实际操作，并返回其结果。
pub fn sys_unlinkat(name: *const u8) -> isize {
    // 获取用户token并转换路径
    let token = current_user_token();
    let name = translated_str(token, name);//将用户态路径指针转换为内核字符串
    if let Some(inode) = ROOT_INODE.find(name.as_str()) {
        if ROOT_INODE.get_link_num(inode.block_id, inode.block_offset) == 1 {
            // clear data if only one link exists
            inode.clear();
        }
        return ROOT_INODE.unlink(name.as_str());
    }
    -1
}
