use core::ffi::c_char;
use bitflags::bitflags;


#[repr(C)]
struct NtfsMbr
{
    jmp_code : [c_char; 3],
    oem_name : [c_char; 8],
    bytes_per_sector : u16,
    sector_per_cluster : u8,
    reserved_sector : [u8; 2],
    unuse1 : [u8; 5],
    media_descriptor : u8,
    unused2 : [u8; 2],
    sectors_per_track : u16,
    megnatic_header_per_cylinder : u16,
    hidden_sectors : u32,
    unused3 : [u8; 4],
    unused4 : u32,
    total_sectors : u64,
    mft_start_cluster : u64,
    size_per_mft : u8,
    unused5 : [u8; 3],
    cluster_per_index : u8,
    unused6 : [u8; 3],
    serial_number : u64,
    check_sum : u32,
    boot_code : [u8; 426],
    end_sign : [u16]
}


#[repr(C)]
pub struct DptByte 
{
    active_partition : u8,
    start_info : [u8; 3],
    partition_type : u8,
    end_info : [u8; 3],
    used_sector : [u8; 4],
    total_sector : [u8; 4],
}

bitflags!
{
    pub struct PartitionType : u8
    {
        const fspt_null_type = 0x00;
        const fsp_fat32 = 0x01;
        const fspt_xenit__root = 0x02;
        const constfspt_xenix_usr = 0x03;
        const fspt_fat16_32m = 0x04;
        const fspt_extended = 0x05;
        const fspt_fat16 = 0x06;
        const fspt_hpfs_ntfs = 0x07;
        const fspt_lan_step = 0xfe;
        const fspt_bbt = 0xff;
    }
}
