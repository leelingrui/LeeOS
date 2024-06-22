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
}