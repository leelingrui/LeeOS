use core::{ffi::c_char, fmt::Write};
use core::fmt;
use core::arch::asm;
use crate::bochs_break;

use super::{io, string};

const MEM_BASE : u64 = 0xb8000;
const MEM_SIZE : u64 = 0x4000;
const MEM_END : u64 = MEM_BASE + MEM_SIZE;
const WIDTH : u16 = 80;
const HEIGHT : u16 = 25;
const ROW_SIZE : u64 = (WIDTH * 2) as u64;
const SCR_SIZE : u64 = ROW_SIZE * HEIGHT as u64;

const NUL : i8 = 0;
const ENQ : i8 = 0x5;
const BEL : i8 = 0x7;
const BS : i8 = 0x8;
const HT : i8 = 0x9;
const LF : i8 = 0xa;
const VT : i8 = 0xb;
const FF : i8 = 0xc;
const CR : i8 = 0xd;
const DEL : i8 = 0x7f;

const ATTR : u8 = 0x7;
const ERASE : u16 = 0x0720;

const CRT_ADDR_REG : u16 = 0x3d4;
const CRT_DATA_REG : u16 = 0x3d5;

const CRT_CURSOR_H : u8 = 0xe;
const CRT_CURSOR_L : u8 = 0xf;
const CRT_START_ADDR_H : u8 = 0xC; // 显示内存起始位置 - 高位
const CRT_START_ADDR_L : u8 = 0xD; // 显示内存起始位置 - 低位

const STYLE : u8 = 0x7;
const BLINK : u8 = 0x80;
const BOLD : u8 = 0x0f;
const UNDER : u8 = 0x0f;

pub static mut CONSOLE : Console = Console::new();

static START_STR : &str = "
 _                _____ _____ 
| |              |  _  /  ___|
| |     ___  ___ | | | \\ `--. 
| |    / _ \\/ _ \\| | | |`--. \\
| |___|  __/  __/\\ \\_/ /\\__/ /
\\_____/\\___|\\___| \\___/\\____/ \n";

pub struct Console
{
    screen : u64,
    screen_size : u64,
    mem_base : u64,
    mem_size : u64,
    mem_end : u64,
    pos : u64,
    x : u16,
    y : u16,
    width : u16,
    height : u16,
    row_size : u16,
    style : u8,
    erase : u16
}

impl Console
{
    pub fn get_screen(&mut self)
    {
        io::outb(CRT_ADDR_REG, CRT_START_ADDR_H);
        self.screen = (io::inb(CRT_DATA_REG) as u64) << 8;
        io::outb(CRT_ADDR_REG, CRT_START_ADDR_L);
        self.screen = io::inb(CRT_DATA_REG) as u64;
        self.screen <<= 1;
        self.screen += MEM_BASE as u64;
    }
    pub fn set_screen(&self)
    {
        io::outb(CRT_ADDR_REG, CRT_START_ADDR_H);
        io::outb(CRT_DATA_REG, ((self.screen - MEM_BASE) >> 9) as u8);
        io::outb(CRT_ADDR_REG, CRT_START_ADDR_L);
        io::outb(CRT_DATA_REG, ((self.screen - MEM_BASE) >> 1) as u8);
    }

    pub fn get_cursor(&mut self)
    {
        io::outb(CRT_ADDR_REG, CRT_CURSOR_H);
        self.pos = (io::inb(CRT_DATA_REG) as u64) << 8;
        io::outb(CRT_ADDR_REG, CRT_CURSOR_H);
        self.pos |= io::inb(CRT_DATA_REG) as u64;
        self.get_screen();
        self.pos <<= 1;
        self.pos += MEM_BASE as u64;
        let delta = (self.pos - self.screen) >> 1;
        self.x = (delta % WIDTH as u64) as u16;
        self.y = (delta / WIDTH as u64) as u16;
    }

    pub unsafe fn write(&mut self, mut buffer : *const i8, cnt : usize) -> usize
    {
        let mut var = 0;
        while var < cnt {
            match *buffer {
                NUL => break,
                BEL => break,
                HT => break,
                FF | VT => self.lf(),
                DEL => self.del(),
                LF => { self.lf(); self.cr()},
                CR => self.cr(),
                BS => self.bs(),
                _ => self.write_chr(*buffer),
            }
            buffer = buffer.offset(1);
            var += 1;
        }
        self.set_cursor();
        return var;
    }

    unsafe fn cr(&mut self)
    {
        self.pos -= (self.x << 1) as u64;
        self.x = 0;
    }

    unsafe fn bs(&mut self)
    {
        if self.x > 0
        {
            self.x -= 1;
            self.pos -= 2;
            *((self.pos + 0xffff800000000000) as *mut u16) = ERASE;
        }
    }

    unsafe fn del(&mut self)
    {
        *((self.pos + 0xffff800000000000) as *mut u16) = ERASE;
    }

    unsafe fn lf(&mut self)
    {
        if self.y + 1 < self.height
        {
            self.y += 1;
            self.pos += self.row_size as u64;
            return;
        }
        self.scroll_up();
    }

    unsafe fn scroll_up(&mut self)
    {
        if self.screen_size as u64 + (self.row_size as u64) + (self.screen as u64) >= self.mem_end as u64
        {
            string::memcpy_s((self.mem_base + 0xffff800000000000) as *mut u8, self.screen_size as usize, (self.screen + 0xffff800000000000) as *mut u8, self.screen_size as usize);
            self.pos -= self.screen - self.mem_base as u64;
            self.screen = self.mem_base;
        }
        self.erase_screen((self.screen + 0xffff800000000000 + self.screen_size) as *mut u16, self.width as u32);
        self.screen += self.row_size as u64;
        self.pos += self.row_size as u64;
        self.set_screen();
    }

    pub const fn new() -> Console
    {
        Console
        {
            mem_end: MEM_BASE + (MEM_SIZE / ROW_SIZE) * ROW_SIZE,
            width: WIDTH,
            height: HEIGHT,
            screen: 0,
            screen_size: (WIDTH * HEIGHT * 2) as u64,
            pos: 0,
            x: 0,
            y: 0,
            row_size: WIDTH * 2,
            style: STYLE,
            mem_base: MEM_BASE,
            mem_size: (MEM_SIZE / ROW_SIZE) * ROW_SIZE,
            erase: ERASE,
        }
    }

    pub fn init(&mut self)
    {
        self.screen = MEM_BASE as u64;
        self.pos = self.mem_base;
        self.x = 0;
        self.y = 0;
        self.set_cursor();
        self.set_screen();
        self.clear_all();
        crate::printk!("{START_STR}");

    }

    unsafe fn write_chr(&mut self, chr : c_char)
    {
        if self.x >= self.width
        {
            self.x -= self.width;
            self.pos -= self.row_size as u64;
            self.lf();
        }
        unsafe {
            *((self.pos | 0xffff800000000000) as *mut i8) = chr;
            self.pos += 1;
            *((self.pos | 0xffff800000000000) as *mut u8) = self.style;
            self.pos += 1;
            self.x += 1;
        }
    }

    pub fn set_cursor(&self)
    {
        io::outb(CRT_ADDR_REG, CRT_CURSOR_H);
        io::outb(CRT_DATA_REG, ((self.pos - self.mem_base) >> 9) as u8);
        io::outb(CRT_ADDR_REG, CRT_CURSOR_L);
        io::outb(CRT_DATA_REG, ((self.pos - self.mem_base) >> 1) as u8);
    }

    pub fn clear_all(&mut self)
    {
        let mut screen_ptr = self.screen as *mut u16;
        while screen_ptr < MEM_END as *mut u16
        {
            screen_ptr = unsafe
            {
                *screen_ptr = self.erase;
                screen_ptr.offset(1)
            }
        }
    }

    pub unsafe fn erase_screen(&mut self, mut start_pos : *mut u16, cnt : u32)
    {
        let mut var = 0;
        while var < cnt {
            *start_pos = self.erase;
            start_pos = start_pos.offset(1);
            var += 1;
        }
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, output_string : &str) ->fmt::Result
    {
        unsafe{
            self.write(output_string.as_ptr() as *const c_char, output_string.len());
        }
        Ok(())
    }
}

pub fn _print(args : fmt::Arguments)
{

    unsafe {
        CONSOLE.write_fmt(args).unwrap()
    }
}

#[no_mangle]
pub unsafe fn console_init()
{
    CONSOLE.init();
}

#[macro_export]
macro_rules! printk {
    ($($arg:tt)*) => 
    ({
        $crate::kernel::console::_print(format_args!($($arg)*))
    });
}

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! logk {
    ($($arg:tt)*) => 
    ({
        $crate::kernel::console::_print(format_args!($($arg)*))
    });
}


// #[macro_export]
// macro_rules! println {
//     () => ($crate::print!("\n"));
//     ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
// }