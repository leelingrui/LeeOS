use core::{ffi::{c_char, c_void}, ptr::null_mut};
use bitflags::bitflags;
use crate::kernel::{sched::get_current_running_process, string::{is_separator, strrsep}};

use super::{file::{Inode, FS, DirEntry, FSType}, ext4::ext4_permission_check};


bitflags!
{
    pub struct FSPermission : u16
    {
        const IRWXU = 0o700;// 宿主可以读、写、执行/搜索
        const IRUSR = 0o400;// 宿主读许可
        const IWUSR = 0o200;// 宿主写许可
        const IXUSR = 0o100;// 宿主执行/搜索许可
        const IRWXG = 0o070; // 组成员可以读、写、执行/搜索
        const IRGRP = 0o040; // 组成员读许可
        const IWGRP = 0o020; // 组成员写许可
        const IXGRP = 0o010; // 组成员执行/搜索许可
        const IRWXO = 0o007; // 其他人读、写、执行/搜索许可
        const IROTH = 0o004; // 其他人读许可
        const IWOTH = 0o002; // 其他人写许可
        const IXOTH = 0o001; // 其他人执行/搜索许可
        const EXEC = Self::IXOTH.bits();
        const READ = Self::IROTH.bits();
        const WRITE = Self::IWOTH.bits();
    }

}

pub fn permission(inode : *mut Inode, perm : FSPermission) -> bool
{
    unsafe
    {
        match (*(*inode).logical_part_ptr).fs_type {
            FSType::None => panic!("unsupport fs type!\n"),
            FSType::Ext4 => ext4_permission_check(inode, perm),
        }
    }

}

pub fn named(path_name : *const c_char, next : &mut *mut c_char) -> *mut Inode
{
    unsafe
    {
        let mut inode;
        let pcb = get_current_running_process();
        let mut left = path_name as *mut c_char;
        if is_separator(*left)
        {
            inode = (*pcb).get_iroot();
            left = left.offset(1);
        }
        else if *left != 0 {
            inode = (*pcb).get_ipwd();
        }
        else {
            return null_mut()
        }
        (*inode).count += 1;
        *next = left;
        if *left == 0
        {
            return inode;
        }
        let right = strrsep(left);
        if !right.is_null() || right < left
        {
            return inode;
        }
        *next = left;
        let mut result_entry = DirEntry::empty();
        loop
        {
            (*inode).find_entry(left, &mut *next, &mut result_entry);
            if result_entry.dir_entry_type == FSType::None
            {
                return null_mut();
            }
            let tmp_inode = FS.get_inode((*inode).dev, result_entry.get_entry_point_to());
            FS.release_inode(inode);
            inode = tmp_inode;
            if (*inode).is_dir() || !permission(inode, FSPermission::EXEC)
            {
                FS.release_inode(inode);
                return null_mut();
            }
            if right == *next
            {
                return inode;
            }
            left = *next;
        }
    }
}

pub fn namei(path : *const c_char) -> *mut Inode
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
        (*dir).find_entry(name, &mut next, &mut entry);
        if entry.dir_entry_type == FSType::None
        {
            return null_mut();
        }
        let inode = FS.get_inode((*dir).dev, entry.get_entry_point_to());
        entry.dispose();
        inode
    }
}