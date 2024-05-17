use core::intrinsics::unlikely;

use super::Err;

pub const EPERM : i64 = 1;	/* Operation not permitted */
pub const ENOENT : i64 = 2;	/* No such file or directory */
pub const ESRCH : i64 = 3;	/* No such process */
pub const EINTR : i64 = 4;	/* Interrupted system call */
pub const EIO : i64 = 5;	/* I/O error */
pub const ENXIO : i64 = 6;	/* No such device or address */
pub const E2BIG : i64 = 7;	/* Argument list too long */
pub const ENOEXEC : i64 = 8;	/* Exec format error */
pub const EBADF : i64 = 9;	/* Bad file number */
pub const ECHILD : i64 = 10;	/* No child processes */
pub const EAGAIN : i64 = 11;	/* Try again */
pub const ENOMEM : i64 = 12;	/* Out of memory */
pub const EACCES : i64 = 13;	/* Permission denied */
pub const EFAULT : i64 = 14;	/* Bad address */
pub const ENOTBLK : i64 = 15;	/* Block device required */
pub const EBUSY : i64 = 16;	/* Device or resource busy */
pub const EEXIST : i64 = 17;	/* File exists */
pub const EXDEV : i64 = 18;	/* Cross-device link */
pub const ENODEV : i64 = 19;	/* No such device */
pub const ENOTDIR : i64 = 20;	/* Not a directory */
pub const EISDIR : i64 = 21;	/* Is a directory */
pub const EINVAL : i64 = 22;	/* Invalid argument */
pub const ENFILE : i64 = 23;	/* File table overflow */
pub const EMFILE : i64 = 24;	/* Too many open files */
pub const ENOTTY : i64 = 25;	/* Not a typewriter */
pub const ETXTBSY : i64 = 26;	/* Text file busy */
pub const EFBIG : i64 = 27;	/* File too large */
pub const ENOSPC : i64 = 28;	/* No space left on device */
pub const ESPIPE : i64 = 29;	/* Illegal seek */
pub const EROFS : i64 = 30;	/* Read-only file system */
pub const EMLINK : i64 = 31;	/* Too many links */
pub const EPIPE : i64 = 32;	/* Broken pipe */
pub const EDOM : i64 = 33;	/* Math argument out of domain of func */
pub const ERANGE : i64 = 34;	/* Math result not representable */
const MAX_ERRNO : i64 = 4095;


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