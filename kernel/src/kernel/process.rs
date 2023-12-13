use core::{arch::asm, ffi::{c_char, c_void}, ptr::{null_mut, null}, alloc::{GlobalAlloc, Layout}, cmp, cell::OnceCell};

use alloc::{collections::{BinaryHeap, btree_map}, vec::Vec};
use crate::{printk, kernel::{global::{self, KERNEL_DATA_IDX, KERNEL_CODE_IDX, set_tss64, GDT, TSS_IDX, KERNEL_TSS}, cpu::get_cpu_number, sched::{self, set_running_process, get_current_running_process}, idle, interrupt}, fs::{file::{STDOUT, Inode, FileStruct, FS}, namei::Fd}, mm::{mm_type::{self, MmapType}, memory::{USER_STACK_TOP, PAGE_SIZE, link_user_page}}, logk, bochs_break};
pub type Priority = u8;
use crate::mm::memory;

use super::{execve, global::{USER_DATA_IDX, USER_CODE_IDX}};
pub type PCB = ProcessControlBlock;
const MAX_PROGRESS_NUM : Pid = 65536;
pub const MAX_PROCSEE_STACK_SIZE : usize = 0x4000000;
static mut TASK_TABLE : [*mut PCB ;MAX_PROGRESS_NUM as usize] = [null_mut(); MAX_PROGRESS_NUM as usize];
static mut WAIT_MAP : btree_map::BTreeMap<Priority, *mut PCB> = btree_map::BTreeMap::new();
static mut IDLE : *mut PCB = null_mut();
static mut PROCESS_ID_SEQ : Pid = 0;
pub const PROCESS_NAME_LEN : usize = 16;
const THREAD_SIZE : usize = 8 * 1024 * 1024;

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
    reserved : [u64; 12],
    rbp : u64,
    rip : u64
}

pub fn sys_yield()
{
    unsafe { schedule() };
}

extern "C" { pub fn interrupt_exit(); }

fn init_thread()
{
    logk!("kernel init!\n");
    task_to_user_mode();
}

type Pid = i32;

union TaskUnion
{
    pcb : core::mem::ManuallyDrop<ProcessControlBlock>,
    stack : [u8; THREAD_SIZE]
}

pub struct ProcessControlBlock
{
    pub kernel_stack : *mut u64,
    pub stack : *mut c_void,
    pub mm : mm_type::MMStruct,
    pub priority : Priority,
    pub jiffies : u32,
    pub name : [c_char; PROCESS_NAME_LEN],
    pub files : Vec<*mut FileStruct>,
    pub uid : u32,
    pub gid : u32,
    pub pid : Pid,
    pub ppid : Pid,
    pub pgid : Pid,
    pub pml4 : *mut memory::Pml4,
    pub wait_pid : Pid,
    pub blocked : u32,
    pub iroot : *mut FileStruct,
    pub ipwd : *mut FileStruct,
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

fn task_to_user_mode()
{
    unsafe
    {
        let pcb = get_current_running_process();
        let pt_regs = ((*pcb).get_process_kernel_stack() as *mut PtRegs).offset(-1);
        (*pt_regs).rax = 0;
        (*pt_regs).rcx = 1;
        (*pt_regs).rdx = 2;
        (*pt_regs).rbx = 3;
        (*pt_regs).rsi = 4;
        (*pt_regs).rdi = 5;
        (*pt_regs).r8 = 6;
        (*pt_regs).r9 = 7;
        (*pt_regs).r10 = 8;
        (*pt_regs).r11 = 9;
        (*pt_regs).r12 = 10;
        (*pt_regs).r13 = 11;
        (*pt_regs).r14 = 12;
        (*pt_regs).r15 = 13;
        (*pt_regs).ss = (USER_DATA_IDX << 3 | 0b11) as u64;
        (*pt_regs).ds = (USER_DATA_IDX << 3 | 0b11) as u16;
        (*pt_regs).es = (USER_DATA_IDX << 3 | 0b11) as u16;
        (*pt_regs).fs = (USER_DATA_IDX << 3 | 0b11) as u16;
        (*pt_regs).gs = 0;
        (*pt_regs).cs = (USER_CODE_IDX << 3 | 0b11) as u64;
        (*pt_regs).rflags = 0 << 12 | 0b10 | 1 << 9;
        logk!("calling init\n");
        execve::sys_execve("bin/init\0".as_ptr() as *const c_char, null_mut(), null_mut());
        panic!("exec /bin/init failure")
    }

}

impl ProcessControlBlock {
    pub fn get_file(&self, fd : Fd) -> *mut FileStruct
    {
        let file_t = self.files.get(fd);
        match file_t {
            Some(x) => *x,
            None => null_mut(),
        }
    }

    pub fn insert_to_fd(&mut self, file_t : *mut FileStruct) -> Fd
    {
        let mut var = 0;
        while var < self.files.len() {
            if self.files[var] == null_mut()
            {
                self.files[var] = file_t;
                return var;
            }
            var += 1;
        }
        self.files.push(file_t);
        return var;
    }

    pub fn get_iroot(&mut self) -> *mut FileStruct
    {
        unsafe {
            self.iroot   
        }
    }

    pub fn get_ipwd(&mut self) -> *mut FileStruct
    {
        unsafe {
            self.ipwd
        }
    }

    pub fn build_task_stack(&mut self)
    {
        unsafe
        {
            let task_frame = ((self.get_process_kernel_stack() as *mut PtRegs).offset(-1) as *mut TaskFrame).offset(-1);
            (*task_frame).r12 = 0xaa55aa55aa55aa55;
            (*task_frame).r13 = 0xaa55aa55aa55aa55;
            (*task_frame).r14 = 0xaa55aa55aa55aa55;
            (*task_frame).r15 = 0xaa55aa55aa55aa55;
            (*task_frame).rbx = task_frame.offset(1) as u64;
            (*task_frame).rip = interrupt_exit as u64;
            self.stack = task_frame as *mut c_void;
        }
    }

    pub fn get_process_kernel_stack(&self) -> *mut c_void
    {
        unsafe {
            ((self as *const Self) as *const TaskUnion).offset(1) as *mut c_void
        }
    }

    pub fn create_task_control_block() -> *mut ProcessControlBlock
    {
        unsafe
        {
            let result = memory::MEMORY_POOL.alloc(Layout::new::<TaskUnion>()) as *mut ProcessControlBlock;
            if result.is_null()
            {
                panic!("system out of memory!");
            }
            (*result) = ProcessControlBlock { kernel_stack:null_mut(), priority: 0, jiffies: 0, name: [0; PROCESS_NAME_LEN], uid: 0, gid: 0, pid: 0, ppid: 0, pgid: 0, pml4: null_mut(), wait_pid: 0, blocked: 0, mm: mm_type::MMStruct::new(result), stack: null_mut(), iroot: null_mut(), ipwd: null_mut(), start_ptregs: PtRegs::default(), files: Vec::new() };
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
            let stack_vma = (*pcb_addr).mm.create_new_mem_area(USER_STACK_TOP.offset(-(MAX_PROCSEE_STACK_SIZE as isize)) as u64, memory::USER_STACK_TOP as u64);
            (*stack_vma).set_prot(MmapType::PROT_READ | MmapType::PROT_WRITE);
            let process_frame = ((alloc::alloc::alloc(Layout::from_size_align(4096, 4096).unwrap()) as *mut c_void).offset(PAGE_SIZE as isize) as *mut TaskFrame).offset(-1);
            link_user_page(USER_STACK_TOP.offset(-(PAGE_SIZE as isize)), (*stack_vma).get_prot());
            (*pcb_addr).stack = process_frame as *mut c_void;
            (*process_frame).rbx = 1;
            (*process_frame).r12 = 2;
            (*process_frame).r13 = 3;
            (*process_frame).r14 = 4;
            (*process_frame).r15 = 5;
            (*process_frame).rbp = 6;
            (*process_frame).rip = func_addr;
            (*pcb_addr).priority = prio;
            (*pcb_addr).ipwd = FS.get_froot();
            (*pcb_addr).iroot = FS.get_froot();
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
        let aim = PCB::create_new_process(init_thread as u64, 1);
        // direct_to_usermode(aim);
    }

}

#[allow(unused)]
unsafe fn direct_to_usermode(pcb : *mut ProcessControlBlock)
{
    task_switch(pcb)
}


unsafe fn task_switch(pcb : *mut ProcessControlBlock)
{
    let process_frame = (*pcb).stack;
    let old_pcb = get_current_running_process();
    let dst_stack = (*pcb).get_process_kernel_stack() as u64;
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
    set_tss64(&mut KERNEL_TSS, dst_stack, dst_stack, dst_stack, dst_stack, dst_stack, dst_stack, dst_stack, dst_stack, dst_stack, dst_stack);
    asm!(
        "mov rsp, rax",
        "mov rbx, [rsp + 0 * 8]",
        "mov r12, [rsp + 1 * 8]",
        "mov r13, [rsp + 2 * 8]",
        "mov r14, [rsp + 3 * 8]",
        "mov r15, [rsp + 4 * 8]",
        "add rsp, 8 * 5",
        "xchg bx, bx",
        in("rax") process_frame
    )
}