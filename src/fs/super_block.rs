use core::{ffi::c_void, alloc::Layout};

use crate::{logk, kernel::device::{get_device, DeviceType, DevT}, fs::file::FS};

use super::file::disk_read;

static mut SUPER_TABLE : [Superblock; SUPER_NR] = [Superblock::empty(); SUPER_NR];
#[derive(Clone, Copy)]
struct Superblock
{

}


impl Superblock {
    const fn empty() -> Self
    {
        Self {  }
    }
}

fn mount_root()
{
    logk!("mounting root file system...\n");
    let device = get_device(2);
    assert!(device.dev_type != DeviceType::Null);
    let sb = read_super_block(2);
    unsafe { FS.load_root_super_block(2, sb) };

}

fn read_super_block(dev : DevT) -> *mut c_void
{
    let sb = unsafe { alloc::alloc::alloc(Layout::new::<[c_void; 1024]>()) };
    let block1 = disk_read(dev, 2, 2);
    unsafe { compiler_builtins::mem::memcpy(sb, block1 as *mut u8, 1024) };
    sb as *mut c_void
}


const SUPER_NR: usize = 0x10;
pub fn super_init()
{
    mount_root();
}