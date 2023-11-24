use alloc::vec::Vec;

use super::process;

pub static mut RUNNING_PROCESS : Vec<*mut process::ProcessControlBlock> = Vec::new();


pub fn set_running_process(pcb : *mut process::ProcessControlBlock)
{
    unsafe
    {
        RUNNING_PROCESS[0] = pcb;
    }
}

pub fn get_current_running_process() -> *mut process::PCB
{
    unsafe
    {
        return RUNNING_PROCESS[0]
    }
}