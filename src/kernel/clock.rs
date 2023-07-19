use super::io::{self, outb, inb};

const OSCILLATOR : u64 = 11932182;
const SPEAKER_REG : u16 = 0x61;
const BEEP_HZ : u16 = 440;
const BEEP_COUNTER : u64 = OSCILLATOR / BEEP_HZ as u64;
const BEEP_MS : u8 = 100;

static mut BEEPING : bool = false;

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