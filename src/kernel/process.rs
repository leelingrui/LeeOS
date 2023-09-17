use core::{arch::asm, ffi::{c_void, c_char}, ptr::null_mut, alloc::{GlobalAlloc, Layout}, mem::size_of};

use crate::{printk, kernel::{process, global, memory::{MEMORY_POOL, PAGE_SIZE}}};

use super::memory;
pub type PCB = ProcessControlBlock;
const MAX_PROGRESS_NUM : u16 = u16::MAX;
static mut TASK_TABLE : [*mut PCB ;MAX_PROGRESS_NUM as usize] = [null_mut(); MAX_PROGRESS_NUM as usize];
#[repr(C, packed)]
#[derive(Default)]
pub struct PtRegs
{
    pub gs : u16,
    pub fs : u16,
    pub es : u16,
    pub ds : u16,
    pub r15 : u64,
    pub r14 : u64,
    pub r13 : u64,
    pub r12 : u64,
    pub r11 : u64,
    pub r10 : u64,
    pub r9 : u64,
    pub r8 : u64,
    pub rbp : u64,
    pub rdi : u64,
    pub rsi : u64,
    pub rbx : u64,
    pub rdx : u64,
    pub rcx : u64,
    pub rax : u64,
    pub error : u64,
    pub rip : u64,
    pub cs : u64,
    pub rflags : u64,
    pub rsp : u64,
    pub ss : u64
}

fn user_function()
{
    let mut var = 0u64;
    loop {
        unsafe { asm!("mov qword ptr [0x11000], 5") };
        var += 1;
    }
}

type Pid = u32;

unsafe fn do_execve(regs : *mut PtRegs, name : *mut c_char, argv : *mut *mut c_char, envp : *mut *mut c_char) -> u64
{
    let dst = memory::MEMORY_POOL.alloc(Layout::for_value(&1024));
    (*regs).rip = dst as u64;
    (*regs).rsp = 0xa00000;
    (*regs).rax = 0x1;
    (*regs).ds = 0;
    (*regs).es = 0;
    compiler_builtins::mem::memcpy(dst as *mut u8, user_function as *mut u8, 1024);
    return 0;
}

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

fn schedule()
{
    let current = running_process();
    
}

impl ProcessControlBlock {
    unsafe fn create_task() -> *mut ProcessControlBlock
    {
        let result = memory::MEMORY_POOL.alloc(Layout::new::<ProcessControlBlock>()) as *mut ProcessControlBlock;
        (*result) = ProcessControlBlock { kernel_stack:null_mut(), priority: 0, jiffies: 0, name: [0; 16], uid: 0, gid: 0, pid: 0, ppid: 0, pgid: 0, pml4: null_mut(), brk: null_mut(), text:null_mut(), data: null_mut(), end: null_mut(), wait_pid: 0, blocked: 0 };
        result
    }
}

pub fn init()
{
    unsafe
    {
        let pcb = running_process();
        let process_frame = pcb.offset((-(memory::PAGE_SIZE as i64) - size_of::<PtRegs>() as i64) as isize) as *mut PtRegs;
        (*process_frame).rax = 0;
        (*process_frame).rcx = 1;
        (*process_frame).rdx = 2;
        (*process_frame).rbx = 3;
        (*process_frame).rsi = 4;
        (*process_frame).rdi = 5;
        (*process_frame).r9 = 6;
        (*process_frame).r10 = 7;
        (*process_frame).r11 = 8;
        (*process_frame).r12 = 9;
        (*process_frame).r13 = 10;
        (*process_frame).r14 = 11;
        (*process_frame).r15 = 12;
        (*process_frame).cs = ((global::USER_CODE_IDX << 3) | 3) as u64;
        (*process_frame).ds = ((global::USER_DATA_IDX << 3) | 3) as u16;
        (*process_frame).es = ((global::USER_DATA_IDX << 3) | 3) as u16;
        (*process_frame).gs = ((global::USER_DATA_IDX << 3) | 3) as u16;
        (*process_frame).fs = ((global::USER_DATA_IDX << 3) | 3) as u16;
        (*process_frame).rsp = memory::USER_STACK_START as u64;
        (*process_frame).rflags = 0 << 12 | 0b10 | 1 << 9;
        (*process_frame).rip = user_function as u64;
        (*process_frame).ss = ((global::USER_DATA_IDX << 3) | 3) as u64;
        printk!("initing task");
        asm!(
            "mov rsp, {frame}",
            frame = in(reg) process_frame as u64
        );
        asm!(
            "xchg bx, bx",
            "mov rax, [rsp + 15 * 8]",
            "mov rcx, [rsp + 14 * 8]",
            "mov rdx, [rsp + 13 * 8]",
            "mov rbx, [rsp + 12 * 8]",
            "mov rsi, [rsp + 11 * 8]",
            "mov rdi, [rsp + 10 * 8]",
            "mov rbp, [rsp + 9 * 8",
            "mov r8, [rsp + 8 * 8]",
            "mov r9, [rsp + 7 * 8]",
            "mov r10, [rsp + 6 * 8]",
            "mov r11, [rsp + 5 * 8]",
            "mov r12, [rsp + 4 * 8]",
            "mov r13, [rsp + 3 * 8]",
            "mov r14, [rsp + 2 * 8]",
            "mov r15, [rsp + 1 * 8]",
            "mov ds, [rsp + 0 * 8 + 6]",
            "mov es, [rsp + 0 * 8 + 4]",
            "mov fs, [rsp + 0 * 8 + 2]",
            "mov gs, [rsp + 0 * 8 + 0]",
            "add rsp, 8 * 17",
            "iretq"
        );
    }


}