use core::{ffi::c_char, alloc::Layout};

use crate::{mm::memory::{self, USER_STACK_TOP}, fs::{namei::{namei, permission, FSPermission}, file::{EOF, FS}}};

use super::{process::{PtRegs, interrupt_exit, PROCESS_NAME_LEN}, sched::get_current_running_process};

pub fn sys_execve(filename : *const c_char, argv : *mut *mut c_char, envp : *mut *mut c_char)
{
    unsafe
    {
        do_execve(filename, argv, envp);
    }
}

unsafe fn do_execve(file_name : *const c_char, argv : *mut *mut c_char, envp : *mut *mut c_char) -> u64
{
    let file_t = namei(file_name);
    if file_t.is_null()
    {
        return EOF;
    }
    if !(*(*file_t).inode).is_file()
    {
        FS.release_file(file_t);
        return EOF;
    }
    if !permission((*file_t).inode, FSPermission::EXEC)
    {
        FS.release_file(file_t);
        return EOF;
    }
    let pcb = get_current_running_process();
    compiler_builtins::mem::memcpy((*pcb).name.as_ptr() as *mut u8, file_name as *const u8, PROCESS_NAME_LEN);
    // copy argv env
    // todo!()

    // release memory
    (*pcb).mm.release_all();

    // load program
    let entry = load_elf(file_t);
    // set heap memory address

    let pt_regs = ((*pcb).get_process_kernel_stack() as *mut PtRegs).offset(-1);
    (*pt_regs).rip = entry as u64;
    (*pt_regs).rsp = USER_STACK_TOP;

    interrupt_exit();
    FS.release_file(file_t);
    return EOF;
}