use core::intrinsics::unlikely;

use super::Err;

pub const EPERM : Err = 1;	/* Operation not permitted */
pub const ENOENT : Err = 2;	/* No such file or directory */
pub const ESRCH : Err = 3;	/* No such process */
pub const EINTR : Err = 4;	/* Interrupted system call */
pub const EIO : Err = 5;	/* I/O error */
pub const ENXIO : Err = 6;	/* No such device or address */
pub const E2BIG : Err = 7;	/* Argument list too long */
pub const ENOEXEC : Err = 8;	/* Exec format error */
pub const EBADF : Err = 9;	/* Bad file number */
pub const ECHILD : Err = 10;	/* No child processes */
pub const EAGAIN : Err = 11;	/* Try again */
pub const ENOMEM : Err = 12;	/* Out of memory */
pub const EACCES : Err = 13;	/* Permission denied */
pub const EFAULT : Err = 14;	/* Bad address */
pub const ENOTBLK : Err = 15;	/* Block device required */
pub const EBUSY : Err = 16;	/* Device or resource busy */
pub const EEXIST : Err = 17;	/* File exists */
pub const EXDEV : Err = 18;	/* Cross-device link */
pub const ENODEV : Err = 19;	/* No such device */
pub const ENOTDIR : Err = 20;	/* Not a directory */
pub const EISDIR : Err = 21;	/* Is a directory */
pub const EINVAL : Err = 22;	/* Invalid argument */
pub const ENFILE : Err = 23;	/* File table overflow */
pub const EMFILE : Err = 24;	/* Too many open files */
pub const ENOTTY : Err = 25;	/* Not a typewriter */
pub const ETXTBSY : Err = 26;	/* Text file busy */
pub const EFBIG : Err = 27;	/* File too large */
pub const ENOSPC : Err = 28;	/* No space left on device */
pub const ESPIPE : Err = 29;	/* Illegal seek */
pub const EROFS : Err = 30;	/* Read-only file system */
pub const EMLINK : Err = 31;	/* Too many links */
pub const EPIPE : Err = 32;	/* Broken pipe */
pub const EDOM : Err = 33;	/* Math argument out of domain of func */
pub const ERANGE : Err = 34;	/* Math result not representable */


pub const ERESTARTSYS : Err = 512;
pub const ERESTARTNOINTR : Err = 513;
pub const ERESTARTNOHAND : Err = 514;	/* restart if no handler.. */
pub const ENOIOCTLCMD : Err = 515;	/* No ioctl command */
pub const ERESTART_RESTARTBLOCK : Err = 516; /* restart by calling sys_restart_syscall */
pub const EPROBE_DEFER : Err = 517;	/* Driver requests probe retry */
pub const EOPENSTALE : Err = 518;	/* open found a stale dentry */
pub const ENOPARAM : Err = 519;	/* Parameter not supported */

/* Defined for the NFSv3 protocol */
pub const EBADHANDLE : Err = 521;	/* Illegal NFS file handle */
pub const ENOTSYNC : Err = 522;	/* Update synchronization mismatch */
pub const EBADCOOKIE : Err = 523;	/* Cookie is stale */
pub const ENOTSUPP : Err = 524;	/* Operation is not supported */
pub const ETOOSMALL : Err = 525;	/* Buffer or request is too small */
pub const ESERVERFAULT : Err = 526;	/* An untranslatable error occurred */
pub const EBADTYPE : Err = 527;	/* Type not supported by server */
pub const EJUKEBOX : Err = 528;	/* Request initiated, but will not complete before timeout */
pub const EIOCBQUEUED : Err = 529;	/* iocb queued, will get completion event */
pub const ERECALLCONFLICT : Err = 530;	/* conflict with recalled state */
pub const ENOGRACE : Err = 531;	/* NFS file lock reclaim refused */



const MAX_ERRNO : Err = 4095;


#[inline(always)]
pub fn is_err<T>(x : *mut T) -> bool
{
    unlikely((x as Err) >= -MAX_ERRNO)
}

#[inline(always)]
pub fn ptr_err<T>(ptr : *mut T) -> Err
{
    ptr as Err
}

#[inline(always)]
pub fn err_ptr<T>(err : Err) -> *mut T
{
    err as *mut T
}