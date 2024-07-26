use crate::kernel::device::{major, minor, mkdev, DevT};



pub fn dev_init()
{
    
}

#[inline(always)]
pub fn old_decode_dev(dev : DevT) -> DevT
{
    mkdev((dev >> 8) & 0xff, dev & 0xff)
}

#[inline(always)]
pub fn old_encode_dev(dev : DevT) -> DevT
{
    major(dev) << 8 | minor(dev)
}

#[inline(always)]
pub fn new_encode_dev(dev : DevT) -> DevT
{
    let ma = major(dev);
    let mi = minor(dev);
    (mi & 0xff) | (ma << 8) | ((mi & !0xff) << 12)
}

#[inline(always)]
pub fn new_decode_dev(dev : DevT) -> DevT
{
    let major = (dev & 0xfff00) >> 8;
    let minor = (dev & 0xff) | ((dev >> 12) & 0xfff00);
    mkdev(major, minor)
}