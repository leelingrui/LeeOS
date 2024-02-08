use core::{ffi::{c_char, c_void}, ptr::null_mut, iter::empty};
use crate::kernel::{sched::get_current_running_process, string::{is_separator, strrsep}};
use super::file::{DirEntry, FSPermission, FSType, FileFlag, FileStruct, Inode, FS};
pub type Fd = usize;


pub fn permission(inode : *mut Inode, perm : FSPermission) -> bool
{
    unsafe
    {
        let process = get_current_running_process();
        let mut mode = (*inode).i_mode.bits();
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

pub fn named(path_name : *const c_char, next : &mut *mut c_char) -> *mut FileStruct
{
    unsafe
    {
        let mut file_t;
        let pcb = get_current_running_process();
        let mut left = path_name as *mut c_char;
        if is_separator(*left)
        {
            file_t = (*pcb).get_iroot();
            left = left.offset(1);
        }
        else if *left != 0 {
            file_t = (*pcb).get_ipwd();
        }
        else {
            return null_mut()
        }
        *next = left;
        if *left == 0
        {
            return file_t;
        }
        let mut right = strrsep(left);
        if right.is_null() || right < left
        {
            return file_t;
        }
        right = right.offset(1);
        *next = left;
        let mut result_entry = DirEntry::empty();
        loop
        {
            (*(*file_t).inode).find_entry(left, &mut *next, &mut result_entry);
            if result_entry.dir_entry_type == FSType::None
            {
                return null_mut();
            }
            let tmp_inode = FS.get_file((*(*file_t).inode).dev, result_entry.get_entry_point_to(), FileFlag::empty());
            FS.release_file(file_t);
            file_t = tmp_inode;
            if !(*(*file_t).inode).is_dir() || !permission((*file_t).inode, FSPermission::EXEC)
            {
                FS.release_file(file_t);
                return null_mut();
            }
            if right == *next
            {
                return file_t;
            }
            left = *next;
        }
    }
}

pub fn namei(path : *const c_char) -> *mut FileStruct
{
    unsafe
    {
        let mut next = null_mut();
        let dir = named(path, &mut next);
        if dir.is_null()
        {
            return null_mut();
        }
        if next.is_null()
        {
            return dir;
        }
        let name = next;
        let mut entry = DirEntry::empty();
        (*(*dir).inode).find_entry(name, &mut next, &mut entry);
        if entry.dir_entry_type == FSType::None
        {
            return null_mut();
        }
        let file_t = FS.get_file((*(*dir).inode).dev, entry.get_entry_point_to(), FileFlag::empty());
        entry.dispose();
        file_t
    }
}