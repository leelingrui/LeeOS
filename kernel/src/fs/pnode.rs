use crate::fs::mount::{Mount, VFSMount, MntFlags};

#[inline(always)]
pub fn set_mnt_shared(mnt : &mut Mount)
{
    mnt.mnt.mnt_flags = mnt.mnt.mnt_flags.difference(MntFlags::SHARED_MASK);
    mnt.mnt.mnt_flags.set(MntFlags::SHARED, true);
}
