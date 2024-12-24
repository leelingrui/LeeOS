use core::{ffi::{c_char, c_void, CStr}, intrinsics::unlikely, iter::empty, ptr::{null_mut, addr_of_mut}, alloc::Layout};
use core::intrinsics::ptr_offset_from_unsigned;
use alloc::string::String;

use crate::kernel::{sched::get_current_running_process, string::strsep, device::DevT};

use super::{dcache::DEntryFlags, file::{FSPermission, FS, FileMode}, inode::Inode, mount::lookup_mnt, path::Path};
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
        *next = path_name.cast_mut();
        let mut left;
        if **next == '\\' as c_char || **next == '/' as c_char
        { 
            path = (*pcb).get_iroot();
            *next = (*next).offset(1);
        }
        else if **next != 0 {
            path = (*pcb).get_ipwd();
        }
        else {
            return Path::empty();
        }
        if **next == 0
        {
            return path;
        }
        left = strsep(addr_of_mut!(*next), "\\/\0".as_ptr() as *const c_char);
        if (*next).is_null()
        { 
            *next = left;
            return path;
        }
        loop
        { 
            let name = String::from(CStr::from_ptr(left.cast()).to_str().unwrap());
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
            
            left = strsep(addr_of_mut!(*next), "\\/\0".as_ptr().cast());
            if (*next).is_null() 
            {
                *next = left;
                return path;
            } 
        }
    }
}

pub fn namei(path_name : *const c_char) -> Path
{
    unsafe
    {
        let name_len = compiler_builtins::mem::strlen(path_name.cast());
        let layout = Layout::from_size_align(name_len, 8).unwrap();
        let tmp_name = alloc::alloc::alloc(layout) as *mut c_char;
        compiler_builtins::mem::memcpy(tmp_name.cast(), path_name.cast(), name_len + 1);
        let mut next = null_mut();
        let mut path = named(tmp_name.cast(), &mut next);
        if path.dentry.is_null()
        {
            alloc::alloc::dealloc(tmp_name.cast(), layout);
            return Path::empty();
        }
        if next.is_null()
        {
            alloc::alloc::dealloc(tmp_name.cast(), layout);
            return path;
        }
        path.dentry = (*path.dentry).look_up(&String::from(CStr::from_ptr(next).to_str().unwrap()));
        alloc::alloc::dealloc(tmp_name.cast(), layout);
        path
    }
}

pub fn sys_mknod(filename : *const c_char, mode : FileMode, dev : DevT)
{
    unsafe
    {
        FS.mknodat(0, filename, mode, dev);
    }
}
