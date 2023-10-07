use core::ffi::c_void;

use crate::{logk, kernel::device::{Device, get_device, DeviceType, DevT}, fs::{super_block, ext4::FileSystem}};

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
    let device = get_device(1);
    assert!(device.dev_type != DeviceType::Null);
    let sb = read_super_block(3);
    FileSystem::load_super_block(sb);
}

fn read_super_block(dev : DevT) -> *mut c_void
{
    disk_read(dev, 1)
}


const SUPER_NR: usize = 0x10;
pub fn super_init()
{
    mount_root();
}