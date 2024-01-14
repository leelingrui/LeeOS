use super::time::{Time, sys_time};

pub const EV_SYN : u16 = 0x00;
pub const EV_KEY : u16 = 0x01;
pub const EV_REL : u16 = 0x02;
pub const EV_ABS : u16 = 0x03;
pub const EV_MSC : u16 = 0x04;
pub const EV_SW : u16 = 0x05;
pub const EV_LED : u16 = 0x11;
pub const EV_SND : u16 = 0x12;
pub const EV_REP : u16 = 0x14;
pub const EV_FF : u16 = 0x15;
pub const EV_PWR : u16 = 0x16;
pub const EV_FF_STATUS : u16 = 0x17;
pub const EV_MAX : u16 = 0x1f;
pub const EV_CNT : u16 = EV_MAX + 1;


#[derive(Clone, Copy, Debug)]
pub struct InputEvent
{
    _time : Time,
    _type : u16,
    _code : u16,
    _value : i32
}

impl InputEvent
{
    pub fn new(_type : u16, _code : u16, _value : i32) -> Self
    {
        Self { _time: sys_time(), _type, _code, _value }
    }

    pub fn get_type(&self) -> u16
    {
        self._type
    }

    pub fn get_code(&self) -> u16
    {
        self._code
    }

    pub fn get_value(&self) -> i32
    {
        self._value
    }
}