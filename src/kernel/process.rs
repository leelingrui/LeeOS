use core::{arch::asm, ffi::c_void};

use super::memory;

pub type PCB = ProcessControlBlock;

type Pid = u32;

pub struct ProcessControlBlock
{
    kernel_stack : *mut u64,
    priority : u32,
    jiffies : u32,
    name : [char; 16],
    uid : u32,
    gid : u32,
    pid : Pid,
    ppid : Pid,
    pgid : Pid,
    pml4 : *mut memory::Pml4,
    brk : *mut c_void,
    text : *mut c_void,
    data : *mut c_void,
    end : *mut c_void,
}

pub fn running_process() -> *mut PCB
{
    let result;
    unsafe { asm!(
            "mov rax, rsp",
            "and rax, 0xfffffffffffff000",
            out("rax") result
        ) }
    result
}