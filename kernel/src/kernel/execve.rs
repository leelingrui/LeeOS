use core::ptr::null_mut;
use core::{ffi::c_char, alloc::Layout, arch::asm};
use core::ffi::c_void;

use crate::fs::file::{FSPermission, FileFlag};
use crate::kernel::process::MAX_PROCSEE_STACK_SIZE;
use crate::kernel::relocation::process_relocation;
use crate::mm::mm_type::MmapType;
use crate::{mm::memory::{self, USER_STACK_TOP}, fs::{namei::{namei, permission}, file::{EOF, FS, sys_write, STDOUT}}, bochs_break, logk};

use super::{process::{PtRegs, interrupt_exit, PROCESS_NAME_LEN}, sched::get_current_running_process, elf64::load_elf64, syscall};

pub fn sys_execve(filename : *const c_char, argv : *mut *mut c_char, envp : *mut *mut c_char)
{
    unsafe
    {
        do_execve(filename, argv, envp);
    }
}

unsafe fn do_execve(file_name : *const c_char, argv : *mut *mut c_char, envp : *mut *mut c_char) -> i64
{
    let pcb = get_current_running_process();
    let pt_regs = ((*pcb).get_process_kernel_stack() as *mut PtRegs).offset(-1);
    let file_t = FS.open_file(file_name, FileFlag::empty());
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
    logk!("prepare load elf file\n");
    compiler_builtins::mem::memcpy((*pcb).name.as_ptr() as *mut u8, file_name as *const u8, PROCESS_NAME_LEN);
    // copy argv env
    // todo!()

    // release memory
    (*pcb).mm.release_all();

    // load program
    let entry = load_elf64(file_t);
    // build user stack area
    let stack_vma = (*pcb).mm.create_new_mem_area(USER_STACK_TOP.offset(-(MAX_PROCSEE_STACK_SIZE as isize)) as u64, memory::USER_STACK_TOP as u64);
    (*stack_vma).set_prot(MmapType::PROT_READ | MmapType::PROT_WRITE);

    // set heap memory address

    (*pt_regs).rip = entry as u64;
    (*pt_regs).rbp = USER_STACK_TOP as u64;
    (*pt_regs).rsp = USER_STACK_TOP as u64;
    asm!(
        "mov rsp, {aim_frame}",
        "jmp [interrupt_exit@GOTPCREL + rip]",
        aim_frame = in(reg) pt_regs as u64
    );
    FS.release_file(file_t);
    return EOF;
}

unsafe fn test() -> !
{
    static TEST : &str = "test!!!\0";
    sys_write(STDOUT, TEST.as_ptr() as *const c_void, TEST.len());
    loop {
        
    }
}