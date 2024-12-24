use core::ptr::{addr_of_mut, null_mut};

use super::{dcache::{self, DEntry, DEntryOperations}, inode::Inode};

pub fn always_delete_dentry(_dentry : &mut DEntry) -> i64
{
    1
}

pub static mut SIMPLE_DENTRY_OPERATIONS : DEntryOperations = DEntryOperations
{
    d_revalidate: None,
    d_weak_revalidate: None,
    d_hash: None,
    d_compare: None,
    d_delete: Some(always_delete_dentry as dcache::DeleteFunc),
    d_init: None,
    d_release: None,
    d_prune: None,
    d_iput: None,
    d_dname: None,
};

pub fn simple_lookup(_dir : *mut Inode, dentry : *mut DEntry, _flags : u64) -> *mut DEntry
{
    unsafe
    {
        (*dentry).d_seq.wrlock();
        if (*(*dentry).d_sb).s_d_op.is_null()
        {
            (*dentry).d_op = addr_of_mut!(SIMPLE_DENTRY_OPERATIONS);
        }
        (*dentry).d_seq.wrunlock();
        return null_mut();
    }
}
