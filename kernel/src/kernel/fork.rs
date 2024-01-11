use core::{ffi::c_void, ptr::null_mut, mem::size_of, arch::asm};

use crate::{mm::memory::{CloneFlags, copy_page_table, Pml4}, bochs_break, logk, kernel::process::{PROCESS_NAME_LEN, PtRegs}};

use super::{process::{Pid, PCB, task_switch}, sched::get_current_running_process, Err};

pub struct KernelCloneArgs
{
    clone_flags : CloneFlags,
    stack_start : *mut c_void,
    stack_size : isize,
}

pub fn sys_vfork() -> Pid
{
    let args = KernelCloneArgs
    {
        clone_flags: CloneFlags::CLONE_VFORK | CloneFlags::CLONE_VM,
        stack_start: null_mut(),
        stack_size: 0,
    };
    kernel_clone(&args)
}

pub fn sys_fork() -> Pid
{
    let args = KernelCloneArgs
    {
        clone_flags: CloneFlags::empty(),
        stack_start: null_mut(),
        stack_size: 0,
    };
    kernel_clone(&args)
}

unsafe fn dup_pcb(src_pcb : *mut PCB, node : u32) -> *mut PCB
{
    bochs_break!();
    let p = PCB::create_new_process(0, 0);
    (*p).stack = (*src_pcb).stack;
    (*p).uid = (*src_pcb).uid;
    (*p).gid = (*src_pcb).gid;
    (*p).ppid = (*src_pcb).pid;
    (*p).uid = (*src_pcb).uid;
    compiler_builtins::mem::memcpy((*p).name.as_ptr() as *mut u8, (*src_pcb).name.as_ptr() as *const u8, PROCESS_NAME_LEN); // copy process name
    compiler_builtins::mem::memcpy((*p).get_intr_frame() as *mut u8, ((*src_pcb).stack as *mut u8).offset(0x30), size_of::<PtRegs>()); // copy return interrupt frame
    (*p).build_task_stack();
    p
}

pub fn copy_process(pid : Pid, trace : u32, node : u32, args : &KernelCloneArgs) -> *mut PCB
{
    unsafe
    {
        let clone_flags = &args.clone_flags;
        let retval;
        let p = dup_pcb(get_current_running_process(), node);
        // check errors
        // todo!()

        retval = copy_mm(clone_flags, p);
        if retval != 0
        {
            return null_mut();
        }
        logk!("copy finished!\n");
        p
    }
}

pub fn kernel_clone(args : &KernelCloneArgs) -> Pid
{
    unsafe
    {
        let p = copy_process(0, 0, u32::MAX, args);
        if p.is_null()
        {
            return -1;
        }
        (*p).insert_to_task_table();
        (*p).pid
    }
}

fn copy_mm(clone_flags : &CloneFlags, dst_pcb : *mut PCB) -> Err
{
    unsafe {
        (*dst_pcb).pml4 = copy_page_table(get_current_running_process(), clone_flags) as *mut Pml4;        
    }
    0
}