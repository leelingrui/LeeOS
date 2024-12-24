use core::{alloc::Layout, ffi::{c_char, c_void, CStr}, intrinsics::unlikely, ptr::{self, drop_in_place}};

use proc_macro::__init;

use crate::{fs::file::FS, kernel::{device::{self, get_device, DevT, DeviceType, DEV_NULL}, errno_base::{is_err, ptr_err, EBUSY, EINVAL}, Err}, logk, printk};

use super::{ext4::{Ext4DirEntry, Ext4DirEntry2}, file::{early_disk_read, FileSystem, LogicalPart}, fs::{SB_ACTIVE, SB_RDONLY}, fs_context::FsContext};


#[__init]
unsafe fn test_fs()
{
    let root = FS.get_froot();
    let mut buffer = alloc::alloc::alloc(Layout::from_size_align_unchecked(4096, 1)) as *mut c_void;
    let _read_size = FS.read_inode((*root.dentry).d_inode, buffer, 4096, 0);
    if _read_size != 0
    {

        loop {
            let dirs = buffer as *mut Ext4DirEntry2;
            if (*dirs).inode != 0
            {
                printk!("file name: {}\n", CStr::from_ptr(&(*dirs).name as *const i8).to_str().unwrap());
                buffer = buffer.offset((*dirs).rec_len as isize);
            }
            else {
                break;
            }
        }
    }
}

#[__init]
fn mount_root()
{
    logk!("mounting root file system...\n");
    // root disk is first part of first disk
    match get_device(259 << 20) {
        Some(device) => 
        {
            // unsafe { FS.load_root_super_block(device.dev, sb) };
        },
        None => panic!("no root file system!\n"),
    }

}

fn setup_bdev_super(sb : &mut LogicalPart, sb_flags : u32, fc : *mut FsContext) -> Err
{
    0
}

pub fn get_tree_bdev(fc : *mut FsContext, fill_super : fn(&mut LogicalPart, &mut FsContext) -> Err) -> Err
{
    unsafe
    {
        if (*fc).source.is_empty()
        {
            return -EINVAL;
        }
        let mut dev = 0;
        let mut err = FS.lookup_bdev((*fc).source.as_ptr() as *mut c_char, &mut dev);
        if 0 != err
        {
            return err;
        }
        let s = FS.sget_dev(fc, dev);
        if is_err(s)
        {
            return ptr_err(s);
        }
        if !(*s).s_root.is_null()
        {
            if unlikely((((*s).s_flags ^ (*fc).sb_flags) & SB_RDONLY) != 0)
            {
                FS.deactive_logic_part(s);
                return -EBUSY
            }
        }
        else {
            err = setup_bdev_super(s, (*fc).sb_flags, fc);
            if 0 == err
            {
                err = fill_super(s, &mut *fc);
            }
            if 0 != err
            {
                FS.deactive_logic_part(s);
                return err;
            }
            (*s).s_flags |= SB_ACTIVE;
        }
        if !(*fc).root.is_null()
        {
            panic!("fc.root already set");
        }
        (*fc).root = (*(*s).s_root).dget();
        0
    }

}

pub fn vfs_get_tree(fc : *mut FsContext) -> Err
{
    unsafe
    {
        if !(*fc).root.is_null()
        {
            return -EBUSY;
        }
        let err = match (*(*fc).ops).get_tree {
            Some(func) => func(&*fc),
            None => panic!("get_tree_cant_be_null!"),
        };
        if err < 0
        {
            return err;
        }
        if unlikely((*fc).root.is_null())
        {
            panic!("Filesystem {} get_tree() didn't set fc->root", (*(*fc).fs_type).name);
        }
        0
    }

}


#[__init]
fn read_super_block(dev : DevT) -> *mut c_void
{
    unsafe
    {
        let sb = alloc::alloc::alloc(Layout::new::<[c_void; 1024]>());
        let block1 = early_disk_read(dev, 2, 2);
        (*block1).read_from_buffer(sb as *mut c_void, 0, 1024);
        (*block1).dispose();
        sb as *mut c_void
    }

}


const SUPER_NR: usize = 0x10;

#[__init]
pub fn super_init()
{
    mount_root();
}
