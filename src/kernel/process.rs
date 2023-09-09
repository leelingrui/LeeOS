use core::{arch::asm, ffi::{c_void, c_char}, ptr::{null, null_mut}};

use super::memory;

pub type PCB = ProcessControlBlock;
const MAX_PROGRESS_NUM : u16 = u16::MAX;
static mut task_table : [PCB ;MAX_PROGRESS_NUM as usize] = [PCB::new() ;MAX_PROGRESS_NUM as usize];

type Pid = u32;

#[derive(Copy, Clone)]
pub struct ProcessControlBlock
{
    kernel_stack : *mut u64,
    priority : u32,
    jiffies : u32,
    name : [c_char; 16],
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
    wait_pid : Pid,
    blocked : u32,
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

impl ProcessControlBlock {
    const fn new() -> ProcessControlBlock
    {
        ProcessControlBlock { kernel_stack:null_mut(), priority: 0, jiffies: 0, name: [0; 16], uid: 0, gid: 0, pid: 0, ppid: 0, pgid: 0, pml4: null_mut(), brk: null_mut(), text:null_mut(), data: null_mut(), end: null_mut(), wait_pid: 0, blocked: 0 }
    }
}