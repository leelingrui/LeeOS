use core::{ffi::{c_char, c_void, CStr}, iter::empty, ptr::null_mut};
use core::intrinsics::ptr_offset_from_unsigned;
use alloc::string::String;

use crate::kernel::{sched::get_current_running_process, string::{is_separator, strsep}};
use super::{dcache::DEntry, file::{DirEntry, FSPermission, FSType, FileFlag, File, FS}, inode::Inode};
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

pub fn named(path_name : *const c_char, next : &mut *mut c_char) -> *mut DEntry
{
    unsafe
    {
        let mut dentry_t;
        let pcb = get_current_running_process();
        let mut left = path_name as *mut c_char;
        if is_separator(*left)
        {
            dentry_t = (*pcb).get_iroot();
            left = left.offset(1);
        }
        else if *left != 0 {
            dentry_t = (*pcb).get_ipwd();
        }
        else {
            return null_mut()
        }
        *next = left;
        if *left == 0
        {
            return dentry_t;
        }
        let mut right = strsep(left);
        if right.is_null() || right < left
        {
            return dentry_t;
        }
        right = right.offset(1);
        *next = left;
        loop
        {
            let name_len = ptr_offset_from_unsigned(right, left) - 1;
            let name = String::from_raw_parts(*next as *mut u8, name_len, name_len);
            dentry_t = (*dentry_t).look_up(&name);
            if dentry_t.is_null()
            {
                return null_mut();
            }
            
            left = right;
            right = strsep(left);

            if right.is_null() || right < left
            {
                *next = left;
                return dentry_t;
            }
        }
    }
}

pub fn namei(path : *const c_char) -> *mut DEntry
{
    unsafe
    {
        let mut next = null_mut();
        let dentry_t = named(path, &mut next);
        if dentry_t.is_null()
        {
            return null_mut();
        }
        if next.is_null()
        {
            return dentry_t;
        }
        (*dentry_t).look_up(&String::from(CStr::from_ptr(next).to_str().unwrap()))
    }
}