use core::{ffi::{c_void, CStr}, alloc::Layout};

use crate::{logk, kernel::device::{get_device, DeviceType, DevT, DEV_NULL, self}, fs::file::FS, printk};

use super::{file::early_disk_read, ext4::{Ext4DirEntry, Ext4DirEntry2}};



unsafe fn test_fs()
{
    let root = FS.get_froot();
    let mut buffer = alloc::alloc::alloc(Layout::from_size_align_unchecked(4096, 1)) as *mut c_void;
    let _read_size = FS.read_inode((*root).d_inode, buffer, 4096, 0);
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

fn mount_root()
{
    logk!("mounting root file system...\n");
    // root disk is first part of first disk
    match get_device(259 << 20) {
        Some(device) => 
        {
            let sb = read_super_block(device.dev);
            unsafe { FS.load_root_super_block(device.dev, sb) };
        },
        None => panic!("no root file system!\n"),
    }

}

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
pub fn super_init()
{
    mount_root();
}