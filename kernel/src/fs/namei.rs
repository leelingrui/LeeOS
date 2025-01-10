use core::{alloc::Layout, ffi::{c_char, c_void, CStr}, intrinsics::unlikely, iter::empty, ptr::{addr_of_mut, null_mut}};
use core::intrinsics::ptr_offset_from_unsigned;
use alloc::string::String;

use crate::kernel::{device::DevT, errno_base::EXDEV, sched::get_current_running_process, string::strsep};

use super::{dcache::DEntryFlags, file::{FSPermission, FileMode, FS}, inode::Inode, mount::{__lookup_mnt, lookup_mnt, mnt_has_parent, real_mount, Mount}, path::Path};
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
            let curr_name = CStr::from_ptr(left.cast()).to_str().unwrap();
            if !handle_dots(&mut path, curr_name)
            {
                (*path.dentry).look_up(curr_name, &mut path);  
            }
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

pub fn handle_dots(path : &mut Path, curr_name : &str) -> bool
{
    unsafe 
    {
        let pcb = get_current_running_process();
        if curr_name == ".."
        {
            if unlikely(path.dentry == (*path.mnt).mnt_root) {
                choose_mountpoint(real_mount(path.mnt), &mut (*pcb).get_iroot(), path);
            }
            else {
                path.dentry = (*path.dentry).get_parent();                
            }
            return true;
        }
        if curr_name == "."
        {
            return true;
        }
        false
    }
}

pub fn namei(path_name : *const c_char) -> Path
{
    unsafe
    {
        let pcb = get_current_running_process();
        let name_len = compiler_builtins::mem::strlen(path_name.cast()) + 1;
        let layout = Layout::from_size_align(name_len, 8).unwrap();
        let tmp_name = alloc::alloc::alloc(layout) as *mut c_char;
        compiler_builtins::mem::memcpy(tmp_name.cast(), path_name.cast(), name_len);
        let mut next = null_mut();
        let mut path = named(tmp_name.cast(), &mut next);
        if path.dentry.is_null() && path.mnt.is_null()
        {
            alloc::alloc::dealloc(tmp_name.cast(), layout);
            return Path::empty();
        }
        if next.is_null()
        {
            alloc::alloc::dealloc(tmp_name.cast(), layout);
            return path;
        }
        let curr_name = CStr::from_ptr(next).to_str().unwrap();
        if !handle_dots(&mut path, curr_name)
        {
            (*path.dentry).look_up(&String::from(curr_name), &mut path);

        }
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

pub fn follow_up(path : &mut Path)
{
    unsafe 
    {
        let mnt = real_mount(path.mnt);
        let parent = (*mnt).mnt_parent;
        if parent == mnt
        {
            return;
        }
        let mountpoint = (*mnt).mnt_mountpoint;
        path.dentry = mountpoint;
        path.mnt = addr_of_mut!((*parent).mnt);
    }

}


fn choose_mountpoint(mut m : *mut Mount, root : &mut Path, path : &mut Path) -> bool
{
    unsafe 
    {
        while mnt_has_parent(m)
        {
            let mountpoint = (*m).mnt_mountpoint;
            m = (*m).mnt_parent;
            if unlikely(root.dentry == mountpoint && root.mnt == addr_of_mut!((*m).mnt))
            {
                break;
            }
            if mountpoint != (*m).mnt.mnt_root
            {
                path.mnt = addr_of_mut!((*m).mnt);
                path.dentry = mountpoint;
                return true;
            }
        }
        false
    }
}