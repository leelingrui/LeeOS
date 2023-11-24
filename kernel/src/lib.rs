#![no_main]
#![feature(ptr_metadata)]
#![feature(int_roundings)]
#![feature(const_trait_impl)]
#![feature(core_intrinsics)]
#![feature(alloc_layout_extra)]
#![feature(allocator_api)]
#![no_std]

extern crate alloc;

pub mod kernel;
pub mod fs;
pub mod mm;