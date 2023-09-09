use core::arch::asm;

static mut IDLE_CNT : u64 = 0;

pub fn idle()
{
    unsafe
    {
        loop {
            IDLE_CNT += 1;
            asm!("hlt");
        }
    }
}