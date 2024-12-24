use core::ptr::{null, null_mut, addr_of_mut};

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

    pub fn head_insert(&mut self, head : &mut Self)
    {
        unsafe
        {
            self.prev = addr_of_mut!(*head);
            self.next = head.next;
            head.next = addr_of_mut!(*self);
        }
    }
}
