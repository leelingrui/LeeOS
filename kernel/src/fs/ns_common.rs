use super::dcache::DEntry;
use core::sync::atomic::AtomicI64;


pub struct NsCommon
{
    pub stashed : *mut DEntry,
    pub count : AtomicI64
}
