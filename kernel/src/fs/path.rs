use core::ptr::null_mut;

use super::{dcache::DEntry, mount::VFSMount};

#[derive(Clone, Copy)]
pub struct Path
{ 
    pub mnt : *mut VFSMount,
    pub dentry : *mut DEntry
}

impl Path
{
    pub fn empty() -> Self
    {
        Self { mnt: null_mut(), dentry: null_mut() }
    }

    pub fn get(&mut self)
    {
        unsafe
        {
            (*self.mnt).mntget();
            (*self.dentry).dget();
        }
    }

    pub fn put(&mut self)
    {   
        unsafe
        {
            (*self.mnt).mntput();
            (*self.dentry).dput();
        }
    }
}
