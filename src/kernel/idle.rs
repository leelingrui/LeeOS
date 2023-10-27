use core::arch::asm;
use crate::{clib::unistd::write, fs::file::STDOUT, logk};
static mut IDLE_CNT : u64 = 0;

pub fn idle()
{
    unsafe
    {
        loop {
            IDLE_CNT += 1;
            logk!("idle!");
            asm!("hlt");
        }
    }
}