use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType,
    EasyFileSystem, DIRENT_SZ,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};
/// Virtual filesystem layer over easy-fs
pub struct Inode {
    inode_id: u32,
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    /// Create a vfs inode
    pub fn new(
        inode_id: u32,
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            inode_id,
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
        for i in 0..file_count {
            let dirent = self.read_dirent(disk_inode, i);
            if dirent.name() == name {
                return Some(dirent.inode_number());
            }
        }
        None
    }

    fn read_dirent(&self, disk_inode: &DiskInode, index: usize) -> DirEntry {
        let mut dirent = DirEntry::empty();
        assert_eq!(
            disk_inode.read_at(index * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
            DIRENT_SZ,
        );
        dirent
    }

    fn insert_dirent(
        &self,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
        name: &str,
        inode_id: u32,
    ) {
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let slot = (0..file_count)
            .find(|&i| self.read_dirent(disk_inode, i).name().is_empty())
            .unwrap_or(file_count);
        if slot == file_count {
            self.increase_size((file_count + 1) as u32 * DIRENT_SZ as u32, disk_inode, fs);
        }
        let dirent = DirEntry::new(name, inode_id);
        assert_eq!(
            disk_inode.write_at(slot * DIRENT_SZ, dirent.as_bytes(), &self.block_device),
            DIRENT_SZ,
        );
    }

    /// Find inode under current inode by name
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        // 目录查找流程：目录 inode -> 遍历 dirent -> 定位子 inode 的磁盘位置。
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    inode_id,
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
        // 先按“新增块数”批量申请数据块，再一次性扩容 inode。
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }

    /// Create inode under current inode by name.
    /// Attention: use find previously to ensure the new file not existing.
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        // 1) 分配新 inode
        let new_inode_id = fs.alloc_inode();
        // 2) 初始化 inode 元数据
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
            });
        // 3) 在当前目录追加 dirent 项
        self.modify_disk_inode(|root_inode| {
            self.insert_dirent(root_inode, &mut fs, name, new_inode_id);
        });

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        // 4) 返回新文件的 Inode 句柄
        Some(Arc::new(Self::new(
            new_inode_id,
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
        // release efs lock automatically by compiler
    }

    /// List inodes by id under current inode
    pub fn readdir(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let dirent = self.read_dirent(disk_inode, i);
                if !dirent.name().is_empty() {
                    v.push(String::from(dirent.name()));
                }
            }
            v
        })
    }

    /// Return the inode number.
    pub fn inode_id(&self) -> u32 {
        self.inode_id
    }

    /// Whether the inode is a directory.
    pub fn is_dir(&self) -> bool {
        self.read_disk_inode(DiskInode::is_dir)
    }

    /// Create a hard link in the current directory.
    pub fn link(&self, name: &str, inode_id: u32) -> isize {
        let mut fs = self.fs.lock();
        let linked = self.modify_disk_inode(|disk_inode| {
            if self.find_inode_id(name, disk_inode).is_some() {
                return false;
            }
            self.insert_dirent(disk_inode, &mut fs, name, inode_id);
            true
        });
        if linked {
            block_cache_sync_all();
            0
        } else {
            -1
        }
    }

    /// Remove a directory entry and return its inode number.
    pub fn unlink(&self, name: &str) -> Option<u32> {
        let _fs = self.fs.lock();
        let removed = self.modify_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            for i in 0..file_count {
                let dirent = self.read_dirent(disk_inode, i);
                if dirent.name() == name {
                    assert_eq!(
                        disk_inode.write_at(
                            i * DIRENT_SZ,
                            DirEntry::empty().as_bytes(),
                            &self.block_device,
                        ),
                        DIRENT_SZ,
                    );
                    return Some(dirent.inode_number());
                }
            }
            None
        });
        if removed.is_some() {
            block_cache_sync_all();
        }
        removed
    }

    /// Count directory entries pointing to the given inode number.
    pub fn count_links(&self, inode_id: u32) -> u32 {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut count = 0;
            for i in 0..file_count {
                let dirent = self.read_dirent(disk_inode, i);
                if !dirent.name().is_empty() && dirent.inode_number() == inode_id {
                    count += 1;
                }
            }
            count
        })
    }

    /// Read data from current inode
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }

    /// Return current inode size in bytes.
    pub fn size(&self) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.size as usize)
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

    /// Deallocate the inode itself from the filesystem bitmap.
    pub fn dealloc(&self) {
        self.fs.lock().dealloc_inode(self.inode_id);
    }
}
