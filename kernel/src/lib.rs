#![no_main]
#![feature(offset_of)]
#![feature(ptr_metadata)]
#![feature(int_roundings)]
#![feature(const_trait_impl)]
#![feature(core_intrinsics)]
#![feature(const_mut_refs)]
#![feature(alloc_layout_extra)]
#![feature(allocator_api)]
#![no_std]
#![allow(warnings, unused)]
extern crate alloc;

pub mod kernel;
pub mod fs;
pub mod mm;
pub mod crypto;
