use core::arch::asm;

pub const __NR_READ : usize = 0;
pub const __NR_WRITE : usize = 1;
pub const __NR_SCHED_YIELD : usize = 24;
pub const __NR_FORK : usize = 57;

pub unsafe fn __syscall0(nr : usize) -> usize
{
    let result;
    asm!(
        "syscall",
        in("rax") nr,
        lateout("rax") result
    );
    result
}

#[inline(always)]
pub unsafe fn __syscall1(nr : usize, arg1 : u64) -> usize
{
    let result;
    asm!(
        "syscall",
        in("rax") nr,
        in("rdi") arg1,
        lateout("rax") result
    );
    result
}

#[inline(always)]
pub unsafe fn __syscall2(nr : usize, arg1 : u64, arg2 : u64) -> usize
{
    let result;
    asm!(
        "syscall",
        in("rax") nr,
        in("rdi") arg1,
        in("rsi") arg2,
        lateout("rax") result
    );
    result
}

#[inline(always)]
pub unsafe fn __syscall3(nr : usize, arg1 : u64, arg2 : u64, arg3 : u64) -> usize
{
    let result;
    asm!(
        "syscall",
        in("rax") nr,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        lateout("rax") result
    );
    result
}

#[inline(always)]
pub unsafe fn __syscall4(nr : usize, arg1 : u64, arg2 : u64, arg3 : u64, arg4 : u64) -> usize
{
    let result;
    asm!(
        "syscall",
        in("rax") nr,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        in("r10") arg4,
        lateout("rax") result
    );
    result
}

#[inline(always)]
pub unsafe fn __syscall5(nr : usize, arg1 : u64, arg2 : u64, arg3 : u64, arg4 : u64, arg5 : u64) -> usize
{
    let result;
    asm!(
        "syscall",
        in("rax") nr,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        in("r10") arg4,
        in("r8") arg5,
        lateout("rax") result
    );
    result
}

#[inline(always)]
pub unsafe fn __syscall6(nr : usize, arg1 : u64, arg2 : u64, arg3 : u64, arg4 : u64, arg5 : u64, arg6 : u64) -> usize
{
    let result;
    asm!(
        "syscall",
        in("rax") nr,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        in("r10") arg4,
        in("r8") arg5,
        in("r9") arg6,
        lateout("rax") result
    );
    result
}