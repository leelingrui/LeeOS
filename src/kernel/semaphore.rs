use core::sync::atomic;

pub struct  SpinLock
{
    counting : atomic::AtomicI64
}

impl SpinLock
{
    pub fn acquire(&mut self, cnt : i64)
    {
        if cnt <= 0 
        {
            panic!()
        }
        let mut expect;
        expect = self.counting.load(atomic::Ordering::Acquire);
        loop
        {
            if expect - cnt >= 0
            {
                match self.counting.compare_exchange_weak(expect, expect - cnt, atomic::Ordering::Release, atomic::Ordering::Relaxed)
                {
                    Ok(_) => break,
                    Err(current) => expect = current,
                }
            }
        }
    }

    pub fn release(&mut self, cnt : i64)
    {
        if cnt <= 0 
        {
            panic!()
        }
        let mut expect = self.counting.load(atomic::Ordering::Acquire);;
        loop
        {
            match self.counting.compare_exchange_weak(expect, expect + cnt, atomic::Ordering::Release, atomic::Ordering::Relaxed)
            {
                Ok(_) => break,
                Err(current) => expect = current,
            }
        }
    }

    pub const fn new(flag : i64) -> SpinLock
    {
        SpinLock {
            counting : atomic::AtomicI64::new(flag)
        }
    }
}