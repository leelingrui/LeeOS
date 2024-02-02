pub mod crc32c;
pub mod crc16;
mod crc32table;

#[macro_export]
macro_rules! tole {
    ($x:expr) => {
        __cpu_to_le32!($x)
    };
}

#[macro_export]
macro_rules! __constant_swap32 {
    ($crc:ident) => {
        (($crc & 0x000000ff) << 24) | (($crc & 0x0000ff00) << 8) | (($crc & 0x00ff0000) >> 8) | (($crc & 0xff000000) >> 24)
    };
    ($crc:expr) => {
        (($crc & 0x000000ff) << 24) | (($crc & 0x0000ff00) << 8) | (($crc & 0x00ff0000) >> 8) | (($crc & 0xff000000) >> 24)
    };
}

#[macro_export]
macro_rules! __le32_to_cpu {
    ($crc:ident) => {
        $crc
    };
    ($crc:expr) => {
        $crc
    };
}

#[macro_export]
macro_rules! __cpu_to_le32 {
    ($crc:ident) => {
        $crc
    };
    ($crc:expr) => {
        $crc
    };
}

#[macro_export]
macro_rules! __be32_to_cpu {
    ($crc:ident) => {
        __constant_swap32!($crc)
    };
    ($crc:expr) => {
        __constant_swap32!($crc)
    };
}

#[macro_export]
macro_rules! __cpu_to_be32 {
    ($crc:ident) => {
        __constant_swap32!($crc)
    };
    ($crc:expr) => {
        __constant_swap32!($crc)
    };
}

