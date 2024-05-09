use core::{alloc::Layout, ffi::{c_char, c_void}, ptr::null_mut, sync::atomic::AtomicI64};

use crate::{kernel::{buffer::Buffer, device::DevT, process::{Gid, Uid, PCB}, Err}, mm::page::Pageflags};

use super::{dcache::DEntry, ext4::{self, ext4_find_entry, ext4_load_all_entries, Ext4Inode, Idx}, file::{DirEntry, FSPermission, FSType, FileFlag, FileMode, LogicalPart}, fs::AddressSpace, mnt_idmapping::MntIdmap};

pub type InodeLoopUp = fn(*mut Inode, *mut DEntry, u64) -> *mut DEntry;
pub type InodeMknode = fn(*mut MntIdmap, *mut Inode, *mut DEntry, FileMode, DevT) -> Err;


pub struct InodeOperations
{
    pub lookup : Option<InodeLoopUp>,
    pub mknod : Option<InodeMknode>
}

pub struct Inode
{
    pub inode_block_buffer : *mut Buffer,
    pub inode_desc_ptr : *mut c_void,
    pub logical_part_ptr : *mut LogicalPart,
    pub address_space : *mut AddressSpace,
    pub i_operations : *const InodeOperations,
    pub count : u32,
    pub i_uid : Uid,
    pub i_gid : Gid,
    pub i_nlink : AtomicI64,
    pub rx_waiter : *mut PCB,
    pub tx_waiter : *mut PCB,
    pub i_perm : FSPermission,
    pub i_mode : FileMode,
    pub dev : DevT,
    pub nr : Idx,
}

impl Inode {
    pub fn new(i_operations : *const InodeOperations, i_perm : FSPermission) -> *mut Self
    {
        unsafe
        {
            let ptr = alloc::alloc::alloc(Layout::new::<Self>()) as *mut Self;
            *ptr = Self { inode_block_buffer: null_mut(), inode_desc_ptr: null_mut(), logical_part_ptr: null_mut(), count: 1, rx_waiter: null_mut(), tx_waiter: null_mut(), dev: 0, nr: 0, i_perm, i_uid: 0, i_gid: 0, i_nlink: AtomicI64::new(1), i_operations, address_space: null_mut(), i_mode: FileMode::empty() };
            ptr
        }
    }

    pub fn inc_nlink(&mut self)
    {
        self.i_nlink.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    }

    pub fn is_dir(&self) -> bool
    {
        self.i_mode.intersects(FileMode::IFDIR)
    }

    pub fn is_file(&self) -> bool
    {
        self.i_mode.intersects(FileMode::IFREG)
    }

    pub fn get_size(&self) -> usize
    {
        unsafe
        {
            match (*self.logical_part_ptr).fs_type {
                FSType::None => panic!("unsupport fs\n"),
                FSType::Ext4 => (*(self.inode_desc_ptr as *mut Ext4Inode)).i_size_lo as usize + (((*(self.inode_desc_ptr as *mut Ext4Inode)).i_size_high as usize) << 32),
                FSType::Shmem => unimplemented!()
            }
        }
    }

    pub fn load_entrys(&mut self, dentry : *mut DEntry)
    {
        unsafe
        {
            match (*self.logical_part_ptr).fs_type {
                FSType::None => panic!("unsupport fs\n"),
                FSType::Ext4 => ext4_load_all_entries(&mut *dentry, self),
                FSType::Shmem => unimplemented!(),
            }
        }

    }

    pub fn find_entry(&mut self, name : *const c_char, next : &mut *mut c_char, result_entry : &mut DirEntry)
    {
        unsafe
        {
            match (*self.logical_part_ptr).fs_type {
                FSType::None => panic!("unsupport fs\n"),
                FSType::Ext4 => ext4_find_entry(self, name, next, result_entry),
                FSType::Shmem => panic!("unsupport fs\n")
            }
        }
    }
}