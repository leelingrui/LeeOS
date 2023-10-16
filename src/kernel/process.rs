use core::{arch::asm, ffi::{c_void, c_char}, ptr::null_mut, alloc::{GlobalAlloc, Layout}, mem::size_of};

use crate::{printk, kernel::{global, cpu::get_cpu_number, sched::{self, set_running_process}}, lib::unistd::write, fs::file::STDOUT, mm::{mm_type, memory::USER_STACK_START}};

use crate::mm::memory;
pub type PCB = ProcessControlBlock;
const MAX_PROGRESS_NUM : u16 = u16::MAX;
const MAX_PROCSEE_STACK_SIZE : usize = 0x4000000;
static mut TASK_TABLE : [*mut PCB ;MAX_PROGRESS_NUM as usize] = [null_mut(); MAX_PROGRESS_NUM as usize];
#[repr(C, packed)]
#[derive(Default)]
#[derive(Clone)]
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
    pub code : u64,
    pub rip : u64,
    pub cs : u64,
    pub rflags : u64,
    pub rsp : u64,
    pub ss : u64
}

pub fn sys_yield()
{
    
}

fn user_function()
{
    let str = "print from usermode\n";
    let mut var = 0u64;
    loop {
        write(STDOUT, str.as_ptr() as *const i8, str.len());
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

pub struct ProcessControlBlock
{
    pub kernel_stack : *mut u64,
    pub pt_regs : PtRegs,
    pub mm : mm_type::MMStruct,
    pub priority : u32,
    pub jiffies : u32,
    pub name : [c_char; 16],
    pub uid : u32,
    pub gid : u32,
    pub pid : Pid,
    pub ppid : Pid,
    pub pgid : Pid,
    pub pml4 : *mut memory::Pml4,
    pub wait_pid : Pid,
    pub blocked : u32,
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
    pub fn create_task_control_block() -> *mut ProcessControlBlock
    {
        unsafe
        {
            let result = memory::MEMORY_POOL.alloc(Layout::new::<ProcessControlBlock>()) as *mut ProcessControlBlock;
            (*result) = ProcessControlBlock { kernel_stack:null_mut(), priority: 0, jiffies: 0, name: [0; 16], uid: 0, gid: 0, pid: 0, ppid: 0, pgid: 0, pml4: null_mut(), wait_pid: 0, blocked: 0, mm: mm_type::MMStruct::new(result), pt_regs: Default::default() };
            result
        }
    }

    pub fn distory_task_control_block(pcb_ptr : *mut ProcessControlBlock)
    {
        unsafe
        {
            memory::MEMORY_POOL.dealloc(pcb_ptr as *mut u8, Layout::new::<ProcessControlBlock>());
        }
    }
}

pub fn init()
{
    unsafe
    {
        printk!("initing task\n");
        sched::RUNNING_PROCESS.resize(get_cpu_number(), null_mut());
        let pcb_addr = ProcessControlBlock::create_task_control_block();
        (*pcb_addr).mm.create_new_mem_area(USER_STACK_START.offset(-(MAX_PROCSEE_STACK_SIZE as isize)) as u64, memory::USER_STACK_START as u64);
        let process_frame = &mut (*pcb_addr).pt_regs;
        process_frame.rax = 0;
        process_frame.rcx = 1;
        process_frame.rdx = 2;
        process_frame.rbx = 3;
        process_frame.rsi = 4;
        process_frame.rdi = 5;
        process_frame.r9 = 6;
        process_frame.r10 = 7;
        process_frame.r11 = 8;
        process_frame.r12 = 9;
        process_frame.r13 = 10;
        process_frame.r14 = 11;
        process_frame.r15 = 12;
        process_frame.cs = ((global::USER_CODE_IDX << 3) | 3) as u64;
        process_frame.ds = ((global::USER_DATA_IDX << 3) | 3) as u16;
        process_frame.es = ((global::USER_DATA_IDX << 3) | 3) as u16;
        process_frame.gs = ((global::USER_DATA_IDX << 3) | 3) as u16;
        process_frame.fs = ((global::USER_DATA_IDX << 3) | 3) as u16;
        process_frame.rsp = USER_STACK_START as u64;// memory::USER_STACK_START as u64;
        process_frame.rflags = 0 << 12 | 0b10 | 1 << 9;
        process_frame.rip = user_function as u64;
        process_frame.ss = ((global::USER_DATA_IDX << 3) | 3) as u64;
        direct_to_usermode(pcb_addr);
    }

}


#[allow(unused)]
unsafe fn direct_to_usermode(pcb : *mut ProcessControlBlock)
{
    let process_frame = &(*pcb).pt_regs as *const PtRegs;
    set_running_process(pcb);
    asm!(
        "mov rsp, {frame}",
        frame = in(reg) process_frame
    );
    asm!(
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
        "add rsp, 8 * 18",
        "xchg bx, bx",
        "iretq"
    );
}