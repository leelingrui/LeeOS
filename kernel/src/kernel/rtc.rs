use crate::kernel::{interrupt::send_eoi, clock::start_beep};

use super::{interrupt::{set_interrupt_handler, IRQ_RTC, set_interrupt_mask, IRQ_CASCADE, self}, io::{outb, CMOS_NMI, CMOS_ADDR_PORT, inb, CMOS_DATA_PROT}};

pub struct RealTimeClock
{

}

impl RealTimeClock
{
    pub fn cmos_read(addr : u8) -> u8
    {
        outb(CMOS_ADDR_PORT, CMOS_NMI | addr);
        let result = inb(CMOS_DATA_PROT);
        result
    }

    pub fn cmos_write(addr : u8, value : u8)
    {
        outb(CMOS_ADDR_PORT, CMOS_NMI | addr);
        outb(CMOS_DATA_PROT, value);
    }

    pub fn init()
    {
        set_interrupt_handler( Self::handler as interrupt::HandlerFn, IRQ_RTC);
        set_interrupt_mask(IRQ_RTC.into(), true);
        set_interrupt_mask(IRQ_CASCADE.into(), true);
    }

    unsafe fn handler(vector : u32)
    {
        assert!(vector == 0x28);
        send_eoi(vector);
        start_beep();
    } 
}
