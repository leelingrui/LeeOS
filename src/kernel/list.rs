use core::ptr::{null, null_mut};

pub struct ListHead
{
    pub next : *mut ListHead,
    pub prev : *mut ListHead,
}

impl ListHead {
    pub fn empty() -> ListHead
    {
        ListHead { next: null_mut(), prev: null_mut() }
    }
}