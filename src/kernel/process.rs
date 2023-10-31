use core::{arch::asm, ffi::{c_char, c_void}, ptr::null_mut, alloc::{GlobalAlloc, Layout}, cmp, cell::OnceCell};

use alloc::collections::{BinaryHeap, btree_map};
use crate::{printk, kernel::{global, cpu::get_cpu_number, sched::{self, set_running_process, get_current_running_process}, idle, interrupt}, clib::unistd::write, fs::file::{STDOUT, Inode}, mm::{mm_type, memory::{USER_STACK_START, PAGE_SIZE}}};
pub type Priority = u8;
use crate::mm::memory;
pub type PCB = ProcessControlBlock;
const MAX_PROGRESS_NUM : Pid = 65536;
const MAX_PROCSEE_STACK_SIZE : usize = 0x4000000;
static mut TASK_TABLE : [*mut PCB ;MAX_PROGRESS_NUM as usize] = [null_mut(); MAX_PROGRESS_NUM as usize];
static mut WAIT_MAP : btree_map::BTreeMap<Priority, *mut PCB> = btree_map::BTreeMap::new();
static mut IDLE : *mut PCB = null_mut();
static mut PROCESS_ID_SEQ : Pid = 0;
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

#[repr(C)]
struct TaskFrame
{
    rbx : u64,
    r12 : u64,
    r13 : u64,
    r14 : u64,
    r15 : u64,
    reserved : [u64; 6],
    rbp : u64,
    rip : u64
}

pub fn sys_yield()
{
    unsafe { schedule() };
}

extern "C" { fn interrupt_exit(); }

fn init_thread()
{
    loop {
        
    }
}

type Pid = i32;

pub struct ProcessControlBlock
{
    pub kernel_stack : *mut u64,
    pub stack : *mut c_void,
    pub mm : mm_type::MMStruct,
    pub priority : Priority,
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
    pub inode : *mut Inode,
    pub ipwd : *mut Inode,
    pub start_ptregs : PtRegs
}

pub fn awake_process(pcb : *mut PCB)
{
    unsafe
    {
        let old = WAIT_MAP.insert((*pcb).priority, pcb);
        if old.is_some()
        {
            panic!("process alread awaken\n");
        }
    }
}

pub unsafe fn schedule()
{
    let current = sched::get_current_running_process();
    if !current.is_null()
    {
        awake_process(current);
    }
    let next_process = WAIT_MAP.pop_first();
    if next_process.is_some()
    {
        if current == next_process.unwrap().1
        {
            return;
        }
        task_switch(next_process.unwrap().1);
    }
    else {
        task_switch(IDLE);
    }
}

impl ProcessControlBlock {
    pub fn create_task_control_block() -> *mut ProcessControlBlock
    {
        unsafe
        {
            let result = memory::MEMORY_POOL.alloc(Layout::new::<ProcessControlBlock>()) as *mut ProcessControlBlock;
            (*result) = ProcessControlBlock { kernel_stack:null_mut(), priority: 0, jiffies: 0, name: [0; 16], uid: 0, gid: 0, pid: 0, ppid: 0, pgid: 0, pml4: null_mut(), wait_pid: 0, blocked: 0, mm: mm_type::MMStruct::new(result), stack: null_mut(), inode: null_mut(), ipwd: null_mut(), start_ptregs: PtRegs::default() };
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

    fn get_avaliable_pid() -> i32
    {
        unsafe
        {
            while !TASK_TABLE[PROCESS_ID_SEQ as usize].is_null() {
                PROCESS_ID_SEQ += 1;
            } 
            PROCESS_ID_SEQ += 1;
            PROCESS_ID_SEQ - 1
            
        }
    }

    pub fn create_new_process(func_addr : u64, prio : Priority) -> *mut PCB
    {
        unsafe
        {
            let pcb_addr = ProcessControlBlock::create_task_control_block();
            (*pcb_addr).mm.create_new_mem_area(USER_STACK_START.offset(-(MAX_PROCSEE_STACK_SIZE as isize)) as u64, memory::USER_STACK_START as u64);
            let process_frame = ((alloc::alloc::alloc(Layout::from_size_align(4096, 4096).unwrap()) as *mut c_void).offset(PAGE_SIZE as isize) as *mut TaskFrame).offset(-1);
            (*pcb_addr).stack = process_frame as *mut c_void;
            (*process_frame).rbx = 1;
            (*process_frame).r12 = 2;
            (*process_frame).r13 = 3;
            (*process_frame).r14 = 4;
            (*process_frame).r15 = 5;
            (*process_frame).rbp = 6;
            (*process_frame).rip = func_addr;
            (*pcb_addr).priority = prio;
            let pid = PCB::get_avaliable_pid();
            TASK_TABLE[pid as usize] = pcb_addr;
            (*pcb_addr).pid = pid;
            WAIT_MAP.insert(prio, pcb_addr);
            pcb_addr
        }
    }
}

pub fn process_init()
{
    unsafe
    {
        printk!("initing task\n");
        IDLE = PCB::create_new_process(idle::idle as u64, 255);
        PCB::create_new_process(init_thread as u64, 1);
    }

}

#[allow(unused)]
unsafe fn direct_to_usermode(pcb : *mut ProcessControlBlock)
{
    set_running_process(pcb);
    asm!(
        "mov rsp, {aim_stackframe}",
        "jmp interrupt_exit",
        aim_stackframe = in(reg) (*pcb).stack
    )
}


unsafe fn task_switch(pcb : *mut ProcessControlBlock)
{
    let process_frame = (*pcb).stack;
    let old_pcb = get_current_running_process();
    set_running_process(pcb);
    if !old_pcb.is_null()
    {
        asm!(
            "sub rsp, 8 * 5",
            "mov [rsp + 0 * 8], rbx",
            "mov [rsp + 1 * 8], r12",
            "mov [rsp + 2 * 8], r13",
            "mov [rsp + 3 * 8], r14",
            "mov [rsp + 4 * 8], r15",
            "mov [rax], rsp",
            in("rax") &((*old_pcb).stack) as *const *mut c_void
        );
    }

    asm!(
        "mov rsp, rax",
        "mov rbx, [rsp + 0 * 8]",
        "mov r12, [rsp + 1 * 8]",
        "mov r13, [rsp + 2 * 8]",
        "mov r14, [rsp + 3 * 8]",
        "mov r15, [rsp + 4 * 8]",
        "add rsp, 8 * 5",
        in("rax") process_frame
    )
}