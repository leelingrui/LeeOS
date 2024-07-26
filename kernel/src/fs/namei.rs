use core::{ffi::{c_char, c_void, CStr}, intrinsics::unlikely, iter::empty, ptr::null_mut};
use core::intrinsics::ptr_offset_from_unsigned;
use alloc::string::String;

use crate::kernel::{sched::get_current_running_process, string::{is_separator, strsep}};

use super::{dcache::DEntryFlags, file::FSPermission, inode::Inode, mount::lookup_mnt, path::Path};
pub type Fd = usize;


pub fn permission(inode : *mut Inode, perm : FSPermission) -> bool
{
    unsafe
    {
        let process = get_current_running_process();
        let mut mode = (*inode).i_perm.bits();
        if (*process).uid == 0
        {
            return true;
        }
        if (*process).uid == (*inode).i_uid as u32
        {
            mode >>= 6;
        }
        else if (*process).gid == (*inode).i_gid as u32 {
            mode >>= 3;
        }
        if (mode & perm.bits() & 0b111) == perm.bits()
        {
            true
        }
        else 
        {
            false
        }
    }
}

pub fn named(path_name : *const c_char, next : &mut *mut c_char) -> Path
{
    unsafe
    {
        let mut path = Path::empty();
        let pcb = get_current_running_process();
        let mut left = path_name as *mut c_char;
        if is_separator(*left)
        {
            path = (*pcb).get_iroot();
            left = left.offset(1);
        }
        else if *left != 0 {
            path = (*pcb).get_ipwd();
        }
        else {
            return Path::empty();
        }
        *next = left;
        if *left == 0
        {
            return path;
        }
        let mut right = strsep(left);
        if right.is_null() || right < left
        {
            return path;
        }
        right = right.offset(1);
        *next = left;
        loop
        {
            let name_len = ptr_offset_from_unsigned(right, left) - 1;
            let name = String::from_raw_parts(*next as *mut u8, name_len, name_len);
            if (*path.dentry).d_flags.contains(DEntryFlags::MOUNTED)
            {
                let mount = lookup_mnt(path.dentry);
                if unlikely(mount.is_null())
                {
                    return Path::empty();
                }
                path.dentry = (*mount).mnt_root;
                path.mnt = mount
            }
            path.dentry = (*path.dentry).look_up(&name);
            if path.dentry.is_null()
            {
                return Path::empty();
            }
            
            left = right;
            right = strsep(left);

            if right.is_null() || right < left
            {
                *next = left;
                return path;
            }
        }
    }
}

pub fn namei(path : *const c_char) -> Path
{
    unsafe
    {
        let mut next = null_mut();
        let mut path = named(path, &mut next);
        if path.dentry.is_null()
        {
            return Path::empty();
        }
        if next.is_null()
        {
            return path;
        }
        path.dentry = (*path.dentry).look_up(&String::from(CStr::from_ptr(next).to_str().unwrap()));
        path
    }
}