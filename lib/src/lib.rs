#![no_main]
#![no_std]
#![feature(start)]
#![feature(linkage)]

pub mod lang_items;
use core::panic::PanicInfo;
use core::ffi::c_char;
pub mod unistd;
use crate::unistd::write;
pub mod syscall_defs;
pub mod macros;
pub mod print;

