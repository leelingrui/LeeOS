use core::{alloc::Layout, ffi::{c_char, c_void, CStr}, ptr::null_mut};

use alloc::{collections::BTreeMap, string::String};

use crate::kernel::{errno_base::{EFAULT, EINVAL, ENOMEM}, Err};

use super::{dcache::DEntry, file::{LogicalPart, FS}, namei::namei};

static mut MOUNT_HLIST : BTreeMap<*mut DEntry, *mut Mount> = BTreeMap::new();
const PATH_MAX : usize = 4096;

pub fn search_mount(dentry : *mut DEntry) -> *mut Mount
{
    unsafe
    {
        match MOUNT_HLIST.get(&dentry) {
            Some(mnt) => *mnt,
            None => null_mut(),
        }
    }
}

pub fn sys_mount(dev_name : *const c_char, dir_name : *const c_char, fstype : *const c_char, flags : u32, data : *const c_void) -> Err
{
    unsafe
    {
        let sdev_name = match CStr::from_ptr(dev_name).to_str() {
            Ok(str) => {
                if str.len() == 0
                {
                    return -EFAULT;
                }
                if str.len() > PATH_MAX
                {
                    return -EINVAL;
                }
                String::from(str)
            },
            Result::Err(_) => return -EINVAL,
        };
        let sdir_name = match CStr::from_ptr(dir_name).to_str() {
            Ok(str) => 
            {
                if str.len() == 0
                {
                    return -EFAULT;
                }
                if str.len() > PATH_MAX
                {
                    return -EINVAL;
                }
                String::from(str)
            },
            Result::Err(_) => return -EINVAL,
        };
        let stype_name = match CStr::from_ptr(fstype).to_str() {
            Ok(str) => 
            {
                if str.len() == 0
                {
                    return -EFAULT;
                }
                if str.len() > PATH_MAX
                {
                    return -EINVAL;
                }
                String::from(str)
            },
            Result::Err(_) => return -EINVAL,
        };
        do_mount(&sdev_name, &sdir_name, &stype_name, flags, data)
    }
}

pub fn path_mount(sb : *mut LogicalPart, dstination : *mut DEntry, mnt_parent : *mut Mount, mnt_devname : &String) -> Err
{
    unsafe
    {
        let mount = Mount::new(sb);
        if mount.is_null()
        {
            return -ENOMEM;
        }
        (*mount).mnt_mp = Mountpoint::new(dstination);
        if (*mount).mnt_mp.is_null()
        {
            return -ENOMEM;
        }
        (*mount).mnt_parent = mnt_parent;
        (*mount).mnt_devname = mnt_devname.clone();
        MOUNT_HLIST.insert(dstination, mount);
        0
    }
}

pub fn do_mount(dev_name : &String, dir_name : &String, fstype : &String, flags : u32, data_page : *const c_void) -> Err
{
    unsafe
    {
        let mount_path = namei(dev_name.as_ptr() as *mut i8);
        let dev_path = namei(dir_name.as_ptr() as *mut i8);
        let mnt_sb = (*dev_path).d_inode as *mut LogicalPart; // todo!()
        let mount = search_mount(mount_path);
        path_mount(mnt_sb, mount_path, mount, dev_name)
    }
}

pub struct Mountpoint
{
    pub m_dentry : *mut DEntry
}

pub struct Mount
{
    pub mnt_parent : *mut Mount,
    pub mnt_mp : *mut Mountpoint,
    pub mnt_devname : String,
    pub mnt : VFSMount
}

pub struct VFSMount
{
    pub mnt_sb : *mut LogicalPart,
    pub mnt_root : *mut DEntry,
    pub mnt_flags : u32
}

impl Mountpoint
{
    fn new(dentry : *mut DEntry) -> *mut Self
    {
        unsafe
        {
            let ptr = alloc::alloc::alloc(Layout::new::<Self>()) as *mut Self;
            (*ptr) = Self { m_dentry: dentry };
            ptr
        }
    }
}

impl Mount {
    fn new(mnt_sb : *mut LogicalPart) -> *mut Self
    {
        unsafe
        {
            let ptr = alloc::alloc::alloc(Layout::new::<Self>()) as *mut Self;
            (*ptr) = Self { mnt_parent: null_mut(), mnt_mp: null_mut(), mnt_devname: String::new(), mnt: VFSMount { mnt_sb, mnt_root: null_mut(), mnt_flags: 0 } };
            ptr
        }
    }
}