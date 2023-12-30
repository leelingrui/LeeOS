use core::{ffi::c_void, ptr::null_mut, mem::size_of};

use crate::mm::memory::{CloneFlags, copy_page_table, Pml4};

use super::{process::{Pid, PCB, task_switch}, sched::get_current_running_process, Err};

pub struct KernelCloneArgs
{
    clone_flags : CloneFlags,
    stack_start : *mut c_void,
    stack_size : isize,
}

pub fn sys_fork() -> Pid
{
    let args = KernelCloneArgs
    {
        clone_flags: CloneFlags::CLONE_VFORK | CloneFlags::CLONE_VM,
        stack_start: null_mut(),
        stack_size: 0,
    };
    kernel_clone(&args)
}

unsafe fn dup_pcb(pcb : *mut PCB, node : u32) -> *mut PCB
{
    let p = PCB::create_new_process(0, 0);
    let pid = (*p).pid;
    let src = get_current_running_process();
    compiler_builtins::mem::memcpy(p as *mut u8, src as *const u8, size_of::<PCB>());
    (*p).pid = pid;
    p
}

pub fn copy_process(pid : Pid, trace : u32, node : u32, args : &KernelCloneArgs) -> *mut PCB
{
    unsafe
    {
        let clone_flags = &args.clone_flags;
        let mut retval = 0;
        let p = dup_pcb(get_current_running_process(), node);
        // check errors
        // todo!()

        retval = copy_mm(clone_flags, p);

        null_mut()
    }
}

pub fn kernel_clone(args : &KernelCloneArgs) -> Pid
{
    unsafe
    {
        let current_pcb = get_current_running_process();
        let new_pcb = PCB::create_new_process(task_switch as u64, 1);
        let p = copy_process(0, 0, u32::MAX, args);
        if current_pcb == get_current_running_process()
        {
            return 0;
        }
        else {
            return (*p).pid;
        }
    }
}

fn copy_mm(clone_flags : &CloneFlags, dst_pcb : *mut PCB) -> Err
{
    unsafe {
        (*dst_pcb).pml4 = copy_page_table(dst_pcb, clone_flags) as *mut Pml4;        
    }
    0
}