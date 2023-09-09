use core::arch::asm;

use crate::logk;

use super::{io::{self, outb, inb}, interrupt};

const OSCILLATOR : u32 = 11932182;
const SPEAKER_REG : u16 = 0x61;
const BEEP_HZ : u16 = 440;
const BEEP_COUNTER : u64 = (OSCILLATOR / BEEP_HZ as u32) as u64;
const BEEP_MS : u8 = 100;
const PIT_CHAN0_REG : u16 = 0x40;
const PIT_CHAN2_REG : u16 = 0x42;
const PIT_CTRL_REG : u16 = 0x43;
const HZ : u32 = 100;
const CLOCK_COUNTER : u32 = OSCILLATOR / HZ;
const IRQ_CLOCK : u8 = 0;

static mut BEEPING : bool = false;
static mut JIFFIES : u64 = 0;

extern "C" fn clock_handler(vector : u32)
{
    assert!(vector == 0x20);
    logk!("clock interrupt occured\n");
    interrupt::send_eoi(vector);
    unsafe { JIFFIES += 1 };
    
}

fn pit_init()
{
    // clock
    io::outb(PIT_CTRL_REG, 0b00110100);
    io::outb(PIT_CHAN0_REG, (CLOCK_COUNTER & 0xff).try_into().unwrap());
    io::outb(PIT_CHAN0_REG, ((CLOCK_COUNTER >> 8) & 0xff).try_into().unwrap());

    // beeper
    io::outb(PIT_CTRL_REG, 0b10110110);
    io::outb(PIT_CHAN2_REG, (BEEP_COUNTER & 0xff).try_into().unwrap());
    io::outb(PIT_CHAN2_REG, ((BEEP_COUNTER >> 8) & 0xff).try_into().unwrap());
}

pub fn clock_init()
{
    pit_init();
    interrupt::regist_irq(clock_handler as interrupt::HandlerFn, IRQ_CLOCK);
    interrupt::get_interrupt_state();
    interrupt::set_interrupt_mask(IRQ_CLOCK.into(), true);
    interrupt::set_interrupt_state(true);
}

fn timer_expires() -> u64
{
    0
}

fn timer_wakeup()
{

}

pub fn start_beep()
{
    unsafe { 
        if BEEPING == false
        {
            io::outb(SPEAKER_REG, io::inb(SPEAKER_REG) | 0b11);
            BEEPING = true;
            let mut var = 0;
            while var < 0xfffff {
                var += 1;
            }
            io::outb(SPEAKER_REG, io::inb(SPEAKER_REG) | 0xfc);
            BEEPING = false;

        }
    }
}