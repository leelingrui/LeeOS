use core::{ffi::c_void, ptr::null_mut};

use crate::{kernel::{Off, sched::get_current_running_process}, fs::{namei::Fd, file::{FileStruct, EOF}}};

use super::mm_type::{VMAreaStruct, MmapType};





pub fn sys_mmap(addr : *const c_void, length : usize, port : MmapType, flags : MmapType, fd : Fd, offset : Off) -> *mut c_void
{
    unsafe
    {
        let pcb = get_current_running_process();
        let file_t = (*pcb).get_file(fd);
        let vma = __do_mmap(addr, length, port, flags, file_t, offset);
        if vma.is_null()
        {
            EOF as *mut c_void
        }
        else {
            (*vma).get_start() as *mut c_void
        }
    }
}
pub fn __do_mmap(addr : *const c_void, length : usize, prot : MmapType, flags : MmapType, file_t : *mut FileStruct, offset : Off) -> *mut VMAreaStruct
{
    unsafe
    {
        assert!((addr as u64 & 0xfff) == 0);
        let pcb = get_current_running_process();
        let vma = (*pcb).mm.scan_empty_space(addr, length, null_mut());
        if !vma.is_null()
        {
            (*vma).set_file(file_t);
            (*vma).set_prot(prot);
            (*vma).set_flags(flags);
            (*vma).set_offset(offset);
            return vma;
        }
        null_mut()
    }
}
