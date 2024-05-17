use core::time;

use crate::{logk, printk};

use super::{rtc::RealTimeClock, io::{CMOS_SECOND, CMOS_MINUTE, CMOS_HOUR, CMOS_DAY, CMOS_MONTH, CMOS_YEAR, CMOS_WEEKDAY, CMOS_CENTURY}, clock::{JIFFIES, JIFFY}};

static mut CENTURY : u32 = 0;
static mut STARTUP_TIME : Time = Time::new();
const MINUTE : u64 = 60;
const HOUR : u64 = 60 * MINUTE;
const DAY : u64 = 24 * HOUR;
const YEAR : u64 = 365 * DAY;
const MONTH : [u64; 12] = [0, 31, 31 + 29, 31 + 29 + 31, 31 + 29 + 31 + 30, 31 + 29 + 31 + 30 + 31, 31 + 29 + 31 + 30 + 31 + 30, 31 + 29 + 31 + 30 + 31 + 30 + 31, 31 + 29 + 31 + 30 + 31 + 30 + 31 + 31, 31 + 29 + 31 + 30 + 31 + 30 + 31 + 31 + 30,  31 + 29 + 31 + 30 + 31 + 30 + 31 + 31 + 30 + 31, 31 + 29 + 31 + 30 + 31 + 30 + 31 + 31 + 30 + 31 + 30 ];
#[repr(C)]
pub struct TM
{
    tm_sec : u32,
    tm_min : u32,
    tm_hour : u32,
    tm_mday : u32,
    tm_mon : u32,
    tm_year : u32,
    tm_wday : u32,
    tm_yday : u32,
    tm_isdst : u32
}

impl TM
{
    pub fn read_bcd() -> Self
    {
        let mut time_bcd = Self {
            tm_sec: 0,
            tm_min: 0, 
            tm_hour: 0,
            tm_mday: 0,
            tm_mon: 0,
            tm_year: 0,
            tm_wday: 0,
            tm_yday: 0,
            tm_isdst: 0,
        };
        time_bcd.tm_sec = RealTimeClock::cmos_read(CMOS_SECOND) as u32;
        time_bcd.tm_min = RealTimeClock::cmos_read(CMOS_MINUTE) as u32;
        time_bcd.tm_hour = RealTimeClock::cmos_read(CMOS_HOUR) as u32;
        time_bcd.tm_mday = RealTimeClock::cmos_read(CMOS_DAY) as u32;
        time_bcd.tm_mon = RealTimeClock::cmos_read(CMOS_MONTH) as u32;
        time_bcd.tm_year = RealTimeClock::cmos_read(CMOS_YEAR) as u32;
        time_bcd.tm_wday = RealTimeClock::cmos_read(CMOS_WEEKDAY) as u32;
        unsafe { CENTURY = RealTimeClock::cmos_read(CMOS_CENTURY) as u32 };
        while time_bcd.tm_sec != RealTimeClock::cmos_read(CMOS_SECOND) as u32 {
            time_bcd.tm_sec = RealTimeClock::cmos_read(CMOS_SECOND) as u32;
            time_bcd.tm_min = RealTimeClock::cmos_read(CMOS_MINUTE) as u32;
            time_bcd.tm_hour = RealTimeClock::cmos_read(CMOS_HOUR) as u32;
            time_bcd.tm_mday = RealTimeClock::cmos_read(CMOS_DAY) as u32;
            time_bcd.tm_mon = RealTimeClock::cmos_read(CMOS_MONTH) as u32;
            time_bcd.tm_year = RealTimeClock::cmos_read(CMOS_YEAR) as u32;
            time_bcd.tm_wday = RealTimeClock::cmos_read(CMOS_WEEKDAY) as u32;
            unsafe { CENTURY = RealTimeClock::cmos_read(CMOS_CENTURY) as u32 };
        }
        time_bcd
    }

    fn bin_to_bcd(bin : u8) -> u8
    {
        (bin % 10) + (bin / 10) * 0x10    
    }

    fn bcd_to_bin(bcd : u8) -> u8
    {
        (bcd & 0xf) + (bcd >> 4) * 10       
    }

    pub fn read() -> Self
    {
        unsafe
        {
            let mut tm = Self::read_bcd();
            tm.tm_sec = Self::bcd_to_bin(tm.tm_sec as u8) as u32;
            tm.tm_min = Self::bcd_to_bin(tm.tm_min as u8) as u32;
            tm.tm_hour = Self::bcd_to_bin(tm.tm_hour as u8) as u32;
            tm.tm_wday = Self::bcd_to_bin(tm.tm_wday as u8) as u32;
            tm.tm_mday = Self::bcd_to_bin(tm.tm_mday as u8) as u32;
            tm.tm_mon = Self::bcd_to_bin(tm.tm_mon as u8) as u32;
            CENTURY = Self::bcd_to_bin(CENTURY as u8) as u32;
            tm.tm_year = Self::bcd_to_bin(tm.tm_year as u8) as u32;
            tm.tm_yday = Self::bcd_to_bin(tm.tm_yday as u8) as u32;
            tm
        }
    }
}


#[derive(Clone, Copy, Debug)]
pub struct Time
{
    pub tick : u64
}

impl Time
{
    pub const fn new() -> Self
    {
        Self {
            tick : 0
        }
    }

    fn leap_year(year : u32) -> bool
    {
        ((year % 4 == 0) && (year % 100 != 0)) || (year % 400 == 0)
    }

    pub fn mktime(time : &TM) -> Time
    {
        unsafe {
            let mut time_stamp = Time::new();
            let year = CENTURY * 100 + time.tm_year;
            let mut base_year = 1970;
            while base_year < year {
                time_stamp.tick += YEAR;
                if Self::leap_year(base_year)
                {
                    time_stamp.tick += DAY;
                }
                base_year += 1;
            }
            time_stamp.tick += MONTH[(time.tm_mon - 1) as usize] * DAY;
            if !Self::leap_year(year) && time.tm_mon > 2
            {
                time_stamp.tick -= DAY;
            }
            time_stamp.tick += (time.tm_mday - 1) as u64 * DAY;
            time_stamp.tick += time.tm_hour as u64 * HOUR;
            time_stamp.tick += time.tm_min as u64 * MINUTE;
            time_stamp.tick += time.tm_sec as u64;
            time_stamp
        }
    }
}

pub fn sys_time() -> Time
{
    unsafe
    {
        Time 
        {
            tick: STARTUP_TIME.tick + (JIFFIES * JIFFY) / 1000,
        }
    }
}


pub fn time_init()
{
    unsafe
    {
        let time = TM::read(); 
        STARTUP_TIME = Time::mktime(&time);
        printk!("start up time: {}{}-{}-{} {}:{}:{}\n", CENTURY, time.tm_year, time.tm_mon, time.tm_mday, time.tm_hour, time.tm_min, time.tm_sec)
    }
}