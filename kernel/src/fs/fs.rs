use core::{alloc::Layout, ffi::c_void, mem::ManuallyDrop, ptr::{self, null_mut}};

use alloc::{collections::BTreeMap, string::String};

use crate::{bit, kernel::{buffer::Buffer, semaphore::RWLock, Err}, mm::page::Pageflags};

use super::{ext4::Idx, file::{FileFlag, LogicalPart}, fs_context::FsContext, inode::Inode};

// mount options
pub const SB_RDONLY : u32 = bit!(0);	/* Mount read-only */
pub const SB_NOSUID : u32 = bit!(1);	/* Ignore suid and sgid bits */
pub const SB_NODEV : u32 = bit!(2);	/* Disallow access to device special files */
pub const SB_NOEXEC : u32 = bit!(3);	/* Disallow program execution */
pub const SB_SYNCHRONOUS : u32 = bit!(4);	/* Writes are synced at once */
pub const SB_MANDLOCK : u32 = bit!(6);	/* Allow mandatory locks on an FS */
pub const SB_DIRSYNC : u32 = bit!(7);	/* Directory modifications are synchronous */
pub const SB_NOATIME : u32 = bit!(10);	/* Do not update access times. */
pub const SB_NODIRATIME : u32 = bit!(11);	/* Do not update directory access times */
pub const SB_SILENT : u32 = bit!(15);
pub const SB_POSIXACL : u32 = bit!(16);	/* Supports POSIX ACLs */
pub const SB_INLINECRYPT : u32 = bit!(17);	/* Use blk-crypto for encrypted files */
pub const SB_KERNMOUNT : u32 = bit!(22);	/* this is a kern_mount call */
pub const SB_I_VERSION : u32 = bit!(23);	/* Update inode I_version field */
pub const SB_LAZYTIME : u32 = bit!(25);	/* Update the on-disk [acm]times lazily */

/* These sb flags are internal to the kernel */
pub const SB_DEAD : u32 = bit!(21);
pub const SB_DYING : u32 = bit!(24);
pub const SB_SUBMOUNT : u32 = bit!(26);
pub const SB_FORCE : u32 = bit!(27);
pub const SB_NOSEC : u32 = bit!(28);
pub const SB_BORN : u32 = bit!(29);
pub const SB_ACTIVE : u32 = bit!(30);
pub const SB_NOUSER : u32 = bit!(31);

//unmount options
pub const MNT_FORCE : u32 = 0x00000001;/* Attempt to forcibily umount */
pub const MNT_DETACH : u32 = 0x00000002;/* Just detach from the tree */
pub const MNT_EXPIRE : u32 = 0x00000004;/* Mark for expiry */
pub const UMOUNT_NOFOLLOW : u32 = 0x00000008;/* Don't follow symlink on umount */
pub const UMOUNT_UNUSED : u32 = 0x80000000;/* Flag guaranteed to be unused */

/* sb->s_iflags */
pub const SB_I_CGROUPWB : u32 = 0x00000001;/* cgroup-aware writeback enabled */
pub const SB_I_NOEXEC : u32 = 0x00000002;/* Ignore executables on this fs */
pub const SB_I_NODEV : u32 = 0x00000004;/* Ignore devices on this fs */
pub const SB_I_STABLE_WRITES : u32 = 0x00000008;/* don't modify blks until WB is done */

/* sb->s_iflags to limit user namespace mounts */
pub const SB_I_USERNS_VISIBLE : u32 = 0x00000010; /* fstype already mounted */
pub const SB_I_IMA_UNVERIFIABLE_SIGNATURE : u32 = 0x00000020;
pub const SB_I_UNTRUSTED_MOUNTER : u32 = 0x00000040;
pub const SB_I_EVM_UNSUPPORTED : u32 = 0x00000080;

pub const SB_I_SKIP_SYNC : u32 = 0x00000100; /* Skip superblock at global sync */
pub const SB_I_PERSB_BDI : u32 = 0x00000200; /* has a per-sb bdi */
pub const SB_I_TS_EXPIRY_WARNED : u32 = 0x00000400; /* warned about timestamp range expiry */
pub const SB_I_RETIRED : u32 = 0x00000800; /* superblock shouldn't be reused */
pub const SB_I_NOUMASK : u32 = 0x00001000; /* VFS does not apply umask */


pub struct FileSystemType
{
    pub name : &'static str,
    pub next : *mut Self,
    pub init_fs_context : Option<fn(*mut FsContext) -> Err>,
    pub fs_supers :  BTreeMap<String, LogicalPart>,
    pub kill_sb : Option<fn(*mut LogicalPart)>
}

pub struct AddressSpace
{
    host : *const Inode,
    i_pages : BTreeMap<Idx, *mut c_void>,
    invalidate_lock : RWLock,
    fgp_mask : Pageflags,
    flags : FileFlag,
    
}

impl AddressSpace
{
    pub fn new(host : *const Inode, fgp_mask : Pageflags, flags : FileFlag) -> *mut Self
    {
        unsafe
        {
            let ptr = alloc::alloc::alloc(Layout::new::<Self>()) as *mut Self;
            *ptr = Self { host, i_pages: BTreeMap::<Idx, *mut c_void>::new(), invalidate_lock: RWLock::new(), fgp_mask, flags };
            ptr
        }
    }

    pub fn destory(&mut self)
    {
        unsafe
        {
            ptr::drop_in_place(self);
            alloc::alloc::dealloc(self as *mut Self as *mut u8, Layout::new::<Self>());
        }
    }

    pub fn seek(&self, idx : Idx) -> *mut c_void
    {
        match self.i_pages.get(&idx) {
            Some(page) => *page,
            None => null_mut(),
        }
    }
}