use crate::kernel::errno_base::ENOSPC;

pub struct Ida 
{
    ids : i32
}


impl Ida
{
    pub const fn new() -> Self
    {
        Self { ids : 0 }
    }

    pub fn alloc_range(&mut self, min : i32, max : i32) -> i32
    {
        if self.ids < min
        {
            self.ids = min + 1;
            return min;
        }
        let result = self.ids;
        if result > max
        {
            return (-ENOSPC) as i32;
        }
        self.ids += 1;
        result
    }

    pub fn alloc_min(&mut self, min : i32) -> i32
    {
        self.alloc_range(min, !0)
    }
}