use core::{ffi::{c_char, c_void}, ptr::null_mut, iter::empty};
use bitflags::bitflags;
use crate::kernel::{sched::get_current_running_process, string::{is_separator, strrsep}};
use super::{file::{Inode, FS, DirEntry, FSType, FileStruct, FileFlag}, ext4::ext4_permission_check};
pub type Fd = usize;


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