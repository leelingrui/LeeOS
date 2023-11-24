use core::sync::atomic;

use super::process::sys_yield;


pub struct  SpinLock
{
    counting : atomic::AtomicI64
}

impl Default for SpinLock {
    fn default() -> Self {
        Self { counting: atomic::AtomicI64::new(0) }
    }
}

pub struct RWLock
{
    reader_num : u64,
    change_mutex : SpinLock,
    writer_mutex : SpinLock
}

impl RWLock {
    pub fn new() -> Self
    {
        Self { reader_num: 0, change_mutex: SpinLock::new(1), writer_mutex: SpinLock::new(1) }
    }

    pub fn rdunlock(&mut self)
    {
        self.change_mutex.acquire(1);
        self.reader_num -= 1;
        self.change_mutex.release(1);
    }

    pub fn wrunlock(&mut self)
    {
        self.writer_mutex.release(1);
        self.change_mutex.release(1);
    }

    pub fn rdlock(&mut self)
    {
        self.writer_mutex.acquire(1);
        self.change_mutex.acquire(1);
        self.reader_num += 1;
        self.change_mutex.release(1);
        self.writer_mutex.release(1);
    }

    pub fn wrlock(&mut self)
    {
        self.writer_mutex.acquire(1);
        loop  {
            self.change_mutex.acquire(1);
            if self.reader_num == 0
            {
                break;
            }
            self.change_mutex.release(1);
            sys_yield();
        }
    }
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

    pub const fn new(start_cnt : i64) -> SpinLock
    {
        SpinLock {
            counting : atomic::AtomicI64::new(start_cnt)
        }
    }
}