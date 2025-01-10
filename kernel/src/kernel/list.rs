use core::ptr::{addr_of, addr_of_mut, null, null_mut};

#[derive(Clone, Copy)]
pub struct ListHead
{
    pub next : *mut ListHead,
    pub prev : *mut ListHead,
}

impl ListHead {
    pub const fn init(&mut self)
    {
        unsafe 
        {
            self.prev = addr_of_mut!(*self);
            self.next = addr_of_mut!(*self);
        }
    }

    pub const fn empty() -> Self
    {
        Self { next: null_mut(), prev: null_mut()  }
    }

    pub fn is_empty(&self) -> bool
    {
        self.next.cast_const() == addr_of!(*self)
    }

    pub fn delete(&mut self)
    {
        unsafe 
        {
            if self.prev.is_null()
            {
                (*self.prev).next = self.next;
            }
            if self.next.is_null()
            {
                (*self.next).prev = self.prev;
            }
            self.prev = addr_of_mut!(*self);
            self.next = addr_of_mut!(*self);
        }

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
    pub fn tail_insert(&mut self, head : &mut Self)
    {
        unsafe
        {
            let tail = head.prev;
            (*tail).next =  addr_of_mut!(*self);
            self.prev = tail;
            head.prev = addr_of_mut!(*self);
            self.next = addr_of_mut!(*head);
        }
    }
}
