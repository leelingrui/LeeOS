pub mod lang_items;
pub mod io;
pub mod console;
pub mod relocation;
pub mod interrupt;
pub mod semaphore;
pub mod string;
pub mod idle;
pub mod global;
pub mod clock;
pub mod bitmap;
pub mod process;
pub mod math;
pub mod list;
pub mod bitops;
pub mod cpu;
pub mod fpu;
pub mod syscall;
pub mod sched;
pub mod time;
pub mod device;
pub mod buffer;
pub mod execve;
pub mod elf64;
pub mod fork;
pub mod keyboard;
pub mod rtc;

pub type Off = usize;
pub type Err = i64;