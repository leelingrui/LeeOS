use core::{ffi::{c_void, CStr, c_char}, alloc::Layout};

use alloc::{vec::Vec, alloc::alloc, string::String};
use proc_macro::__init;

use crate::{fs::ext4::Idx, mm::memory::PAGE_SIZE};

use super::device::{DEV_CMD_SECTOR_COUNT, DEV_CMD_SECTOR_START, DevT, device_install, regist_device, DeviceIoCtlFn, DeviceWriteFn, DeviceReadFn};

const SECTOR_SIZE : usize = 0x1000;
static mut RAMDISKS : Vec<RamDisk> = Vec::new();

struct RamDisk
{
    start : *mut c_void,
    length : usize
}

impl RamDisk {
    fn ioctl(disk : *mut RamDisk, cmd : i64, _args : *mut c_void, flags : u32) -> i64
    {
        unsafe
        {
            match cmd {
                DEV_CMD_SECTOR_START => 0,
                DEV_CMD_SECTOR_COUNT =>
                {
                    ((*disk).length / SECTOR_SIZE) as i64
                },
                _ => { panic!("unknow device command: {}", cmd) }
            }
        }
    }
    
    fn read(disk : *mut RamDisk, start_block : Idx, num_blocks : usize, buf : *mut c_void, flags : u32)
    {
        unsafe
        {
            let start_addr = (*disk).start.offset(start_block as isize * SECTOR_SIZE as isize);
            let len = num_blocks as usize * SECTOR_SIZE;
            assert!(start_addr.offset(len as isize) > (*disk).start.offset((*disk).length as isize));
            compiler_builtins::mem::memcpy(buf as *mut u8, start_addr as *const u8, len);
        }
    }

    fn write(disk : *mut RamDisk, idx : Idx, count : usize, buf : *mut c_void, flags : u32)
    {
        unsafe
        {
            let start_addr = (*disk).start.offset(idx as isize * SECTOR_SIZE as isize);
            let len = count as usize * SECTOR_SIZE;
            assert!(start_addr.offset(len as isize) > (*disk).start.offset((*disk).length as isize));
            compiler_builtins::mem::memcpy(start_addr as *mut u8, buf as *const u8, len);
        }
    }

    pub fn create(size : usize) -> DevT
    {
        unsafe
        {
            if size & 0xfff != 0
            {
                return 0;
            }
            RAMDISKS.push(Self { start: alloc(Layout::from_size_align(size, PAGE_SIZE).unwrap()) as *mut c_void, length: size  });
            match RAMDISKS.last() {
                Some(disk) => 
                {
                    let mut name = String::new();
                    let _ = core::fmt::write(&mut name, format_args!("ram{}\0", RAMDISKS.len() - 1));
                    device_install(1, disk as *const RamDisk as *mut c_void, CStr::from_ptr(name.as_ptr() as *const c_char), 0, 0)
                },
                None => return 0
            }
        }
    }
}

#[__init]
pub fn ramdisk_init()
{
    unsafe
    {
        regist_device(1, Some(core::mem::transmute::<*mut(), DeviceIoCtlFn>(RamDisk::ioctl as *mut())),
         Some(core::mem::transmute::<*mut(), DeviceReadFn>(RamDisk::read as *mut())),
        Some(core::mem::transmute::<*mut(), DeviceWriteFn>(RamDisk::write as *mut())), None)
    }
}