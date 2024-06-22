use core::ptr::{null, null_mut};

#[derive(Clone, Copy)]
pub struct ListHead
{
    pub next : *mut ListHead,
    pub prev : *mut ListHead,
}

impl ListHead {
    pub const fn empty() -> ListHead
    {
        ListHead { next: null_mut(), prev: null_mut() }
    }

    pub fn is_empty(&self) -> bool
    {
        self.next as *const Self == self as *const Self
    }
}