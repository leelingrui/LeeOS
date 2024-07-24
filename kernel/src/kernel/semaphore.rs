use core::sync::atomic;
use alloc::collections::BTreeMap;
use super::process::{sys_yield, PCB};
use super::sched::get_current_running_process;

pub struct  SpinLock
{
    counting : atomic::AtomicI64,
}

impl Default for SpinLock {
    fn default() -> Self {
        Self { counting: atomic::AtomicI64::new(0) }
    }
}

pub struct RWLock
{
    readers : BTreeMap<*const PCB, i64>,
    change_mutex : SpinLock,
    writer_mutex : SpinLock
}

impl RWLock {
    pub fn new() -> Self
    {
        Self { readers: BTreeMap::new(), change_mutex: SpinLock::new(1), writer_mutex: SpinLock::new(1) }
    }

    pub fn rdunlock(&mut self)
    {
        self.change_mutex.acquire(1);
        let pcb = get_current_running_process().cast_const();
        match self.readers.get_mut(&pcb)
        {
            Some(container) => 
            {
                *container -= 1;
                if *container == 0
                {
                    self.readers.remove(&pcb);
                }
            },
            None =>
            {
                panic!("you can't unlock from other thread!");
            }
        }
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
        let pcb = get_current_running_process().cast_const();
        match self.readers.get_mut(&pcb)
        {
            Some(container) => 
            {
                *container += 1;
            },
            None =>
            {
                self.readers.insert(pcb, 1);
            }
        }
        self.change_mutex.release(1);
        self.writer_mutex.release(1);
    }

    pub fn wrlock(&mut self)
    {
        self.writer_mutex.acquire(1);
        loop {
            self.change_mutex.acquire(1);
            let pcb = get_current_running_process().cast_const();
            match self.readers.get_mut(&pcb)
            {
                Some(_) =>
                {
                    if self.readers.len() == 1
                    {
                        break;
                    }
                    sys_yield();
                },
                None => break
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
            else
            {
                expect = self.counting.load(atomic::Ordering::Acquire);
            }
        }
    }

    pub fn release(&mut self, cnt : i64)
    {
        if cnt <= 0 
        {
            panic!()
        }
        let mut expect = self.counting.load(atomic::Ordering::Acquire);
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
