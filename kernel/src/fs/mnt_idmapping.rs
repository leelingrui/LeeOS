use core::{alloc::Layout, sync::atomic::AtomicU64};

pub static mut NOP_MNT_IDMAP : MntIdmap = MntIdmap
{
    count: AtomicU64::new(1)
};

pub struct MntIdmap
{
    count : AtomicU64
}

impl MntIdmap {
    pub fn new() -> *mut Self
    {
        unsafe
        {
            let ptr = alloc::alloc::alloc(Layout::new::<Self>()) as *mut Self;
            *ptr = Self { count: AtomicU64::new(1) };
            ptr
        }
    }
}
