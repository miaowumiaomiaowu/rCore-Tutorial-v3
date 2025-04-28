use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType,
    EasyFileSystem, DIRENT_SZ,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};
/// Virtual filesystem layer over easy-fs

#[allow(missing_docs)]
pub struct Inode {
    pub block_id: usize,
    pub block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    /// Create a vfs inode
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }
    /// Call a function over a disk inode to read it
    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }
    /// Call a function over a disk inode to modify it
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }
    /// Find inode under a disk inode by name
    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some(dirent.inode_id() as u32);
            }
        }
        None
    }
    /// Find inode under current inode by name
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }
    /// Increase the size of a disk inode
    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }

    /// Create inode under current inode by name
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        let op = |root_inode: &DiskInode| {
            // assert it is a directory
            assert!(root_inode.is_dir());
            // has the file been created?
            self.find_inode_id(name, root_inode)
        };
        if self.read_disk_inode(op).is_some() {
            return None;
        }
        // create a new file
        // alloc a inode with an indirect block
        let new_inode_id = fs.alloc_inode();
        // initialize inode
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
            });
        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        // return inode
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
        // release efs lock automatically by compiler
    }

    /// Create hard link, only ROOT_NODE can call it
    /// sys_linkat会调用这个函数
    pub fn link(&self, old: &str, new: &str) -> Option<Arc<Inode>> {
        //  获取底层 EasyFileSystem 实例的锁。这个实例包含了文件系统的元数据（如 inode 分配位图、数据块位图等），加锁是为了保证操作的原子性。
        let mut fs = self.fs.lock();
        // 定义一个闭包 op，用于查找 old 文件名对应的 inode ID。它会在 ROOT_INODE 的磁盘镜像 (DiskInode) 上执行。
        let op = |root_inode: &DiskInode| {
            // assert it is a directory
            assert!(root_inode.is_dir());
            // has the file been created?
            self.find_inode_id(old, root_inode)//遍历 ROOT_INODE（作为目录）的所有目录项（DirEntry），查找名为 old 的条目。如果找到，返回其对应的 inode ID (u32)。
        };
        // self.read_disk_inode(op):读取ROOT_INODE对应的磁盘块到块缓存，并在缓存中执行闭包op（即调用 self.find_inode_id(old, root_inode)）。
        if let Some(old_inode_id) =  self.read_disk_inode(op) {
            // We need to keep old inode and new inode has the same 'block_id' and 'block_offset'.
            // Thus we can create a hard link.
            let new_inode_id = old_inode_id;// 这是硬链接的核心，新的目录项将指向与old文件完全相同的inode ID
            let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);// 获取这个 inode 在磁盘上的具体位置（块号和块内偏移）。
            
            //准备修改ROOT_INODE的磁盘镜像，这会获取块缓存中对应块的可变引用
            self.modify_disk_inode(|root_inode| {
                // 计算目录当前的文件数 file_count 和增加一个条目后的新大小 new_size。
                let file_count = (root_inode.size as usize) / DIRENT_SZ;
                let new_size = (file_count + 1) * DIRENT_SZ;
                //  如果需要，为目录分配新的数据块并更新其大小元数据。
                self.increase_size(new_size as u32, root_inode, &mut fs);
                // 创建一个新的目录项结构，包含新文件名 new 和旧文件的 inode ID new_inode_id。
                let dirent = DirEntry::new(new, new_inode_id);

                //将新的目录项 dirent 写入到 ROOT_INODE 数据区的末尾
                root_inode.write_at(
                    file_count * DIRENT_SZ,
                    dirent.as_bytes(),
                    &self.block_device,
                );
            });

            //创建一个新的 VFS Inode 对象（指向同一个物理 inode），并将其包裹在 Some 中返回给 sys_linkat。
            Some(Arc::new(Self::new(
                new_inode_block_id,
                new_inode_block_offset,
                self.fs.clone(),
                self.block_device.clone(),
            )))
        } else {
            // the old dir doesn't exist
            None
        }
    }

    /// Unlink
    pub fn unlink(&self, name: &str) -> isize {
        // 1. 获取文件系统锁，保证原子性
        let _fs = self.fs.lock();// _fs 未使用，但持有锁直到函数结束

        // 2. 定义查找操作: 检查文件是否存在于当前目录 (self)
        let op = |root_inode: &DiskInode| {
            // assert it is a directory
            assert!(root_inode.is_dir());// 确认是目录
            // has the file been created?
            self.find_inode_id(name, root_inode) // 在目录中查找名为 name 的 inode ID
        };
        // 3. 检查文件是否存在
        //    read_disk_inode 执行查找操作 op
        if let Some(_) = self.read_disk_inode(op) {
            // 文件找到了，可以执行删除操作

            // 4. 修改目录的 DiskInode
            self.modify_disk_inode(|root_inode| {
                let mut buf = DirEntry::empty();// 缓冲区，用于读取目录项
                let mut swap = DirEntry::empty();// 缓冲区，用于暂存最后一个目录项
                let file_count = (root_inode.size as usize) / DIRENT_SZ;// 目录中的条目总数

                // 5. 遍历目录项查找目标
                for i in 0..file_count {
                    // 读取第 i 个目录项到 buf
                    if root_inode.read_at(DIRENT_SZ * i, buf.as_bytes_mut(), &self.block_device) == DIRENT_SZ {
                         // 检查是否是我们要删除的目录项
                        if buf.name() == name {
                            // 找到了！

                            // 6. 用最后一个目录项覆盖当前项
                            //    读取最后一个目录项 (索引 file_count - 1) 到 swap
                            root_inode.read_at(DIRENT_SZ *(file_count - 1), swap.as_bytes_mut(), &self.block_device);
                            // 将 swap (最后一个目录项) 写入到当前位置 i
                            root_inode.write_at(DIRENT_SZ * i, swap.as_bytes_mut(), &self.block_device);
                            // 7. 更新目录大小
                            //    将目录的大小减少一个目录项的大小，逻辑上删除了最后一个条目
                            //    （因为它的内容已经被复制到了位置 i）
                            root_inode.size -= DIRENT_SZ as u32;
                            // 8. 【重要】缺失的 nlink 处理和数据回收
                            //    根据实验要求 "不考虑使用 unlink 彻底删除文件的情况"，
                            //    这里 *没有* 读取被删除目录项指向的 inode，
                            //    也 *没有* 减少该 inode 的硬链接计数 (nlink)，
                            //    更 *没有* 在 nlink 变为 0 时回收 inode 和数据块。
                            //    这简化了实现，但与标准 unlink 行为不同。
                            break; // 找到并处理完，退出循环
                        }
                    }
                }
            });
            // 成功删除目录项，返回 0
            0
        } else {
           // 文件未找到，返回 -1
            -1
        }
        // 文件系统锁在此处自动释放
    }

    /// get link number of thn given file
    pub fn get_link_num(&self, block_id: usize, block_offset: usize) -> u32 {
        let fs = self.fs.lock();
        let mut count = 0;
        self.read_disk_inode(|root_inode| {
            let mut buf = DirEntry::empty();
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            for i in 0..file_count {
                assert_eq!(
                    root_inode.read_at(DIRENT_SZ * i, buf.as_bytes_mut(), &self.block_device),
                    DIRENT_SZ,
                );
                let (this_inode_block_id, this_inode_block_offset) = fs.get_disk_inode_pos(buf.inode_id());
                if this_inode_block_id as usize == block_id && this_inode_block_offset == block_offset {
                    count += 1;
                }
            }
        });
        count
    }

    /// List inodes under current inode
    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                v.push(String::from(dirent.name()));
            }
            v
        })
    }
    /// Read data from current inode
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }
    /// Write data to current inode
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }
    /// Clear the data in current inode
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
        block_cache_sync_all();
    }
}
