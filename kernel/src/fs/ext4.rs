use core::{alloc::Layout, cmp::min, ffi::{c_char, c_void}, mem::{offset_of, size_of}, panic, ptr::{addr_of_mut, drop_in_place, null_mut}, sync::atomic::AtomicI64};

use alloc::{alloc::{alloc, dealloc}, collections::BTreeMap, string::{String, ToString}, vec::Vec};
use proc_macro::__init;

use crate::{crypto::{crc16::crc16, crc32c::{crc32c_le, reverse32, reverse8}}, fs::file::{FSType, EOF, FS}, kernel::{bitmap::BitMap, buffer::{self, Buffer}, device::{device_ioctl, DevT, DEV_CMD_SECTOR_COUNT}, errno_base::EINVAL, io::SECTOR_SIZE, math::{self, log2, pow}, sched::get_current_running_process, string::{memset, EOS}, time::sys_time, Err}, mm::memory::PAGE_SIZE};

use super::{dcache::DEntry, dev::{new_decode_dev, old_decode_dev}, file::{disk_read, early_disk_read, DirEntry, FSPermission, FileMode, FileSystem, LogicalPart}, fs::{FileSystemType, FileSystemFlags}, fs_context::{self, FsContext, FsContextOperations}, inode::Inode, namei::namei, super_block::get_tree_bdev};



pub type Idx = u64; 


const EXT4_NAME_LEN : usize = 0xff;
const EXT4_LABEL_MAX : usize = 0x10;
/*
 * Constants relative to the data blocks
 */
const EXT4_NDIR_BLOCKS : usize = 12;
const EXT4_IND_BLOCK : usize = EXT4_NDIR_BLOCKS;
const EXT4_DIND_BLOCK : usize = EXT4_IND_BLOCK + 1;
const EXT4_TIND_BLOCK : usize = EXT4_DIND_BLOCK + 1;
const EXT4_N_BLOCKS	: usize = EXT4_TIND_BLOCK + 1;
const EXT4_DIRECT_BLOCK : u64 = 12;
const EXT4_INDIRECT1_BLOCK : u64 = 12 + 1024;
const EXT4_INDIRECT2_BLOCK : u64 = 12 + 1024 * 1024;
const EXT4_INDIRECT3_BLOCK : u64 = 12 + 1024 * 1024 * 1024;

const RO_COMPAT_SPARSE_SUPER : i32 = 0x1;
const RO_COMPAT_LARGE_FILE : i32 = 0x2;
const RO_COMPAT_BTREE_DIR : i32 = 0x4;
const RO_COMPAT_HUGE_FILE : i32 = 0x8;
const RO_COMPAT_GDT_CSUM : i32 = 0x10;
const RO_COMPAT_DIR_NLINK : i32 = 0x20;
const RO_COMPAT_EXTRA_ISIZE : i32 = 0x40;
const RO_COMPAT_HAS_SNAPSHOT : i32 = 0x80;
const RO_COMPAT_QUOTA : i32 = 0x100;
const RO_COMPAT_BIGALLOC : i32 = 0x200;
const RO_COMPAT_METADATA_CSUM : i32 = 0x400;
const RO_COMPAT_REPLICA : i32 = 0x800;
const RO_COMPAT_READONLY : i32 = 0x1000;
const RO_COMPAT_PROJECT : i32 = 0x2000;
const RO_COMPAT_VERITY : i32 = 0x8000;
const RO_COMPAT_ORPHAN_PRESENT : i32 = 0x10000;


const DEF_HASH_VERSION_LEGACY : u8 = 0;
const DEF_HASH_VERSION_HALF_MD4 : u8 = 0;
const DEF_HASH_VERSION_TEA : u8 = 0;
const DEF_HASH_VERSION_ULAGACY : u8 = 0;
const DEF_HASH_VERSION_UHALF_MD4 : u8 = 0;
const DEF_HASH_VERSION_UTEA : u8 = 0;


pub struct Ext4GroupDescInfo
{
    pub logical_block_bitmap : BitMap,
    pub inode_bitmap : BitMap,
    pub group_desc_ptr : *mut c_void,
    pub group_desc_no : u32,
    pub parent : *const LogicalPart,
    pub inode_table_offset : Idx,
    pub data_block_start : usize
}

pub static mut EXT4_FS_TYPE : FileSystemType = FileSystemType
{
    name: "ext4\0",
    next: null_mut(),
    init_fs_context: Some(ext4_init_fs_context),
    fs_supers: BTreeMap::new(),
    kill_sb: Some(ext4_kill_sb),
    fs_flags: FileSystemFlags::from_bits_truncate(FileSystemFlags::REQUIRE_DEV.bits() | FileSystemFlags::ALLOW_IDMAP.bits())
};

#[__init]
pub fn ext4_init_fs()
{
    unsafe
    {
        FS.register_filesystem(addr_of_mut!(EXT4_FS_TYPE));
    }
}

pub fn ext4_kill_sb(sb : *mut LogicalPart)
{
    unsafe
    {
        let sbi = (*sb).s_sbi as *mut Ext4SuperBlockInfo;
        FS.kill_block_super(sb);
        drop_in_place(sbi);
        alloc::alloc::dealloc(sbi as *mut u8, Layout::new::<Ext4SuperBlock>());
    }
}

pub static mut EXT4_CONTEXT_OPS : FsContextOperations = FsContextOperations
{
    parse_param: None,
    get_tree: Some(ext4_get_tree),
    parse_monolithic: None
};

pub fn ext4_init_fs_context(fs_context : *mut FsContext) -> Err
{
    unsafe
    {
        (*fs_context).ops = addr_of_mut!(EXT4_CONTEXT_OPS);
        0
    }
}

impl Ext4GroupDescInfo {
    fn new(parent : &LogicalPart) -> Self
    {
        Self { logical_block_bitmap: BitMap::null_bitmap(), inode_bitmap: BitMap::null_bitmap(), group_desc_ptr: null_mut(), group_desc_no: 0, parent: parent as *const LogicalPart, inode_table_offset: 0, data_block_start: 0 }
    }

    fn load_bitmaps(&mut self)
    {
        unsafe
        {
            match (*self.parent).old_fs_type {
                FSType::Ext4 => 
                {
                    assert!(!self.group_desc_ptr.is_null());
                    ext4_load_inode_bitmaps(self);
                    ext4_load_block_bitmap(self);
                },
                FSType::None => panic!("unknow filesystem!"),
                FSType::Shmem => todo!(),
            }

        }
    }
}

impl Ext4SuperBlockInfo
{
    pub fn new() -> *mut Self
    {
        unsafe
        {
            let ptr = alloc::alloc::alloc(Layout::new::<Self>()).cast::<Self>();
            ptr.write(Self
            {
                super_block: null_mut(), group_desc: Vec::new(), inode_per_group: 0, blocks_per_group: 0, inode_count: 0, s_csum_seed: 0
            });
            ptr
        }
    }
}

pub struct Ext4SuperBlockInfo
{
    pub super_block : *mut c_void,
    pub group_desc : Vec<Ext4GroupDescInfo>,
    pub inode_per_group : usize,
    pub blocks_per_group : usize,
    pub inode_count : usize,
    pub s_csum_seed : u32
}

pub fn ext4_get_tree(fc : *mut FsContext) -> Err
{
    return get_tree_bdev(fc, ext4_fill_super)
}

#[inline(always)]
pub fn ext4_inode_per_blocks(sbi : *const Ext4SuperBlockInfo) -> Idx
{
    unsafe
    {
        1024 * (*sbi).inode_per_group as Idx / 256
    }
}


#[inline(always)]
fn ext4_get_group_desc_no(sbi : *const Ext4SuperBlockInfo, nr : Idx) -> Idx
{
    unsafe {
        (nr - 1) as Idx / (*sbi).inode_per_group as Idx        
    }
}

#[inline(always)]
pub fn ext4_get_inode_logical_block(logical_part : &LogicalPart, mut nr : Idx) -> Idx
{

    unsafe {
        nr = nr / ext4_inode_per_blocks(logical_part.s_sbi as *const Ext4SuperBlockInfo) + 1;
        let sb = logical_part.s_sbi as *mut Ext4SuperBlock;
        let desc = &(*(logical_part.s_sbi as *mut Ext4SuperBlockInfo)).group_desc[ext4_get_group_desc_no(logical_part.s_sbi as *const Ext4SuperBlockInfo, nr) as usize];
        (desc.inode_table_offset + nr % (*sb).s_blocks_per_group as u64 / ext4_inode_per_blocks(logical_part.s_sbi as *const Ext4SuperBlockInfo)) as Idx
    }
}

pub fn ext4_permission_check(inode : *mut Inode, perm : FSPermission) -> bool
{
    unsafe
    {
        let desc = (*inode).inode_desc_ptr as *const Ext4Inode;
        let process = get_current_running_process();
        let mut mode = (*desc).i_mode;
        if (*process).uid == 0
        {
            return true;
        }
        if (*process).uid == (*desc).i_uid as u32
        {
            mode >>= 6;
        }
        else if (*process).gid == (*desc).i_gid as u32 {
            mode >>= 3;
        }
        if (mode & perm.bits() & 0b111) == perm.bits()
        {
            true
        }
        else 
        {
            false
        }
    }

}


pub fn ext4_match_name(name : *const c_char, entry_name : *const c_char, next : &mut *mut c_char) -> bool
{
    unsafe
    {
        let mut lhs = name;
        let mut rhs = entry_name;
        while *lhs == *rhs && *lhs != EOS && *rhs != EOS {
            lhs = lhs.offset(1);
            rhs = rhs.offset(1);
        }
        if *rhs != EOS
        {
            return false;
        }
        if *lhs != EOS && todo!()
        {
            return false;
        }
        if todo!()
        {
            lhs = lhs.offset(1);
        }
        *next = lhs as *mut c_char;
        true
    }
}

pub fn write_super_block_check_sum(sb : *mut Ext4SuperBlock)
{
    unsafe
    {
        let result = crc32c_le(!0, sb as *mut c_void, size_of::<Ext4SuperBlock>() - 4);
        (*sb).s_checksum = result as i32;
    }
}

bitflags::bitflags! {
    struct Ext4FileMode : u16
    {
        const IFMT = 0o170000;  // 文件类型（8 进制表示）
        const IFREG = 0o100000;  // 常规文件
        const IFBLK = 0o60000;  // 块特殊（设备）文件，如磁盘 dev/fd0
        const IFDIR = 0o40000;  // 目录文件
        const IFCHR = 0o20000;  // 字符设备文件
        const IFIFO = 0o10000;  // FIFO 特殊文件
        const IFLNK = 0o120000;  // 符号连接
        const IFSOCK = 0o140000; // SOCKET file
    }
}

// n is power of p
fn is_power_of_n(mut n : i64, p : i64) -> bool
{
    if p == 0 && p == 1
    {
        return n == p;
    }
    while n != 0 && ((n % p) == 0) {
        n /= p;
    }
    return n == 1;
}

fn ext4_load_super(sb : &mut LogicalPart) -> Err
{
    unsafe
    {
        let es = alloc::alloc::alloc(Layout::new::<[c_void; 1024]>());
        let block1 = early_disk_read(sb.s_dev, 2, 2);
        sb.s_sbi = Ext4SuperBlockInfo::new().cast();
        (*block1).read_from_buffer(es as *mut c_void, 0, 1024);
        (*sb.s_sbi.cast::<Ext4SuperBlockInfo>()).super_block = es.cast();
        (*block1).dispose();
        0
    }
}

fn ext4_load_group_desc(dev : DevT, idx : Idx) -> *mut Ext4GroupDesc
{
    unsafe
    {
        let desc = alloc::alloc::alloc(Layout::new::<Ext4GroupDesc>()) as *mut Ext4GroupDesc;
        let src = disk_read(dev, 8 + (idx / size_of::<Ext4GroupDesc>() as Idx), 1);
        (*src).read_from_buffer(desc as *mut c_void, size_of::<Ext4GroupDesc>() * (idx as usize % (SECTOR_SIZE as usize / size_of::<Ext4GroupDesc>())), size_of::<Ext4GroupDesc>());
        desc
    }

}

pub fn ext4_fill_super(lp : *mut LogicalPart, fs_context : *mut FsContext) -> Err
{
    unsafe
    {
        let dentry = namei((*fs_context).source.as_ptr() as *mut c_char);
        ext4_load_super(&mut *lp);
        let raw_super_block = (*((*lp).s_sbi as *mut Ext4SuperBlockInfo)).super_block as *mut Ext4SuperBlock;
        let result = crc32c_le(!0, raw_super_block as *const c_void, size_of::<Ext4SuperBlock>()); // check crc;
        if result != 0
        { 
            return -EINVAL;
        }
        (*lp).old_fs_type = FSType::Ext4;
        (*lp).logic_block_size = pow(2.0, (*raw_super_block).s_log_block_size as f64) as i32;
        (*lp).logic_block_count = (((*raw_super_block).s_blocks_count_hi as usize) << 32) + (*raw_super_block).s_blocks_count_lo as usize;
        (*lp).inode_count = (*raw_super_block).s_inodes_count as usize;
        let sbi = (*lp).s_sbi as *mut Ext4SuperBlockInfo;
        (*sbi).inode_per_group = (*raw_super_block).s_inodes_per_group as usize;
        (*sbi).blocks_per_group = (*raw_super_block).s_blocks_per_group as usize;
        (*sbi).s_csum_seed = crc32c_le(!0, (*raw_super_block).s_uuid.as_ptr() as *const c_void, 16);
        let mut var = 0;
        let group_num = ((*raw_super_block).s_blocks_count_lo as i64 + (((*raw_super_block).s_blocks_count_hi as i64) << 32)).div_ceil((*raw_super_block).s_blocks_per_group as i64);
        // init group desc
        while var < group_num {
            let desc = ext4_load_group_desc((*lp).s_dev, var as Idx);
            let group_desc_checksum = ext4_group_desc_csum(&*lp, 0 as u32, desc);
            if group_desc_checksum != (*desc).bg_checksum as u16
            {
                return -EINVAL;
            }
            let sbi = (*lp).s_sbi as *mut Ext4SuperBlockInfo;
            (*sbi).group_desc.push(Ext4GroupDescInfo::new(&*lp));
            let new_desc = (*sbi).group_desc.last_mut().unwrap();
            new_desc.group_desc_ptr = desc as *mut c_void;
            new_desc.load_bitmaps();
            new_desc.inode_table_offset = (((*desc).bg_inode_table_hi as u64) << 32) + (*desc).bg_inode_table_lo as u64;
            new_desc.data_block_start = new_desc.inode_table_offset as usize + math::upround((*raw_super_block).s_inodes_per_group as u64 * 256, (*lp).logic_block_size as u64 * 1024) as usize / ((*lp).logic_block_size as usize * 1024);
            var += 1;
        }
        let inode =(*lp).iget(2);
        (*lp).s_root = DEntry::make_root(inode);
        if !(*lp).s_root.is_null()
        {
            return 0;
        }
        todo!();
    }
}

pub fn ext4_flax_group_init(dev : DevT, sb : *mut Ext4SuperBlock)
{
    unsafe
    {
        let mut first_group_desc_of_flex_groups;
        let mut left_blocks = (((*sb).s_blocks_count_hi as i64) << 32) + (*sb).s_blocks_count_lo as i64;
        let current_group_blocks = (*sb).s_blocks_per_group as i64;
        let current_group_free_inodes = (*sb).s_inodes_per_group as i64;
        let mut block_no = 0;
        let group_desc_buffer = alloc(Layout::new::<[Buffer; 3]>()) as *mut Buffer;
        let inode_table_desc = group_desc_buffer.offset(1);
        let block_table_desc = group_desc_buffer.offset(2);

        *group_desc_buffer = Buffer::new(PAGE_SIZE);
        let group_number = left_blocks.div_ceil(32768);
        let mut group_no = 0;
        let group_bach = pow(2.0, (*sb).s_log_block_size as f64) as usize * 2;
        let flex_batch = pow(2.0, (*sb).s_log_groups_per_flex as f64) as i64;
        while group_no < group_number {
            // init group desc
            let group_desc = ((*group_desc_buffer).buffer as *mut Ext4GroupDesc).offset(group_no as isize % 64);
            let mut current_group_free_blocks = if left_blocks > (*sb).s_blocks_per_group as i64
            {
                (*sb).s_blocks_per_group as i64
            }
            else
            {
                left_blocks
            };
            let current_group_free_inodes = if current_group_free_blocks * 16 < current_group_free_inodes
            {
                current_group_free_blocks * 16
            }
            else
            {
                current_group_free_inodes
            };
            let mut used_blocks = current_group_free_inodes / 16;
            let mut first_block_bitmap = 0;
            if (group_no % flex_batch) == 0
            {
                used_blocks += flex_batch * 2 + group_number;
                first_group_desc_of_flex_groups = group_desc;
                first_block_bitmap = 2 + group_no;
            }
            

            (*group_desc).bg_block_bitmap_lo = (first_block_bitmap & 0xffffffff) as u32;
            (*group_desc).bg_block_bitmap_hi = ((first_block_bitmap >> 32) & 0xffffffff) as i32;
            let first_inode_bitmap = first_block_bitmap + group_bach as i64;
            (*group_desc).bg_inode_bitmap_lo = (first_inode_bitmap & 0xffffffff) as u32;
            (*group_desc).bg_inode_bitmap_hi = ((first_inode_bitmap >> 32) & 0xffffffff) as i32;
            (*group_desc).bg_free_blocks_count_lo = (current_group_free_blocks & 0xffffff) as u16;
            (*group_desc).bg_free_blocks_count_hi = ((current_group_free_blocks >> 32) & 0xffffffff) as i16;
            (*group_desc).bg_free_inodes_count_lo = (current_group_free_inodes & 0xffffff) as u16;
            (*group_desc).bg_free_inodes_count_hi = ((current_group_free_inodes >> 32) & 0xffffffff) as i16;
            (*group_desc).bg_used_dirs_count_lo = 0;
            (*group_desc).bg_used_dirs_count_hi = 0;
            (*group_desc).bg_flags = 0;

            (*group_desc).bg_exclude_bitmap_lo = 0;
            (*group_desc).bg_exclude_bitmap_hi = 0;

            (*group_desc).bg_itable_unused_lo = 0;
            (*group_desc).bg_itable_unused_hi = 0;


            if (group_no % 64) == 63
            {
                // sync to device
                (*group_desc_buffer).write_to_device(dev, 1 * group_bach as u64, group_bach);

                (*group_desc_buffer).write_to_device(dev, 32768 * group_bach as u64, group_bach);
                let mut back_up_blocks = 3;
                while back_up_blocks < group_number {
                    (*group_desc_buffer).write_to_device(dev, back_up_blocks as Idx * 32768 * group_bach as u64, group_bach);
                    back_up_blocks *= 3;
                }
                let mut back_up_blocks = 5;
                while back_up_blocks < group_number {
                    (*group_desc_buffer).write_to_device(dev, back_up_blocks as Idx * 32768 * group_bach as u64, group_bach);
                    back_up_blocks *= 5;
                }
                let mut back_up_blocks = 7;
                while back_up_blocks < group_number {
                    (*group_desc_buffer).write_to_device(dev, back_up_blocks as Idx * 32768 * group_bach as u64, group_bach);
                    back_up_blocks *= 7;
                }
            }
            current_group_free_blocks -= current_group_blocks;
            group_no += 1;
        }
        dealloc(group_desc_buffer as *mut u8, Layout::new::<[Buffer; 3]>());
    }
}

pub fn devmkfs(dev : DevT, mut icount : usize)
{
    unsafe
    {
        let total_block = device_ioctl(dev, DEV_CMD_SECTOR_COUNT, null_mut(), 0) / 4096;
        let blocks_count = total_block;
        let buf = alloc(Layout::new::<Buffer>()) as *mut Buffer;
        *buf = Buffer::new(4096);
        memset((*buf).buffer as *mut u8, 0, 4096);
        // init superblock
        let sb = (*buf).buffer.offset(1024) as *mut Ext4SuperBlock;
        if icount == 0
        {
            icount = ((blocks_count).div_floor(8) * 8) as usize;
        }
        (*sb).s_inodes_count = icount as u32;
        (*sb).s_blocks_count_lo = ((total_block) & 0xffffffff) as u32;
        (*sb).s_blocks_count_hi = ((total_block) >> 32) as u32;
        (*sb).s_r_blocks_count_lo = ((total_block) & 0xffffffff) as u32;
        (*sb).s_r_blocks_count_lo = ((total_block) >> 32) as u32;
        (*sb).s_free_blocks_count_lo = ((total_block - blocks_count) & 0xffffffff) as u32;
        (*sb).s_free_blocks_count_hi = (((total_block - blocks_count) >> 32) & 0xffffffff) as u32;
        (*sb).s_free_inodes_count = icount as i32;


        (*sb).s_log_block_size = 2;
        (*sb).s_log_cluster_size = 2;
        (*sb).s_inodes_per_group = 8 * SECTOR_SIZE as i32 * pow(2.0, (*sb).s_log_block_size as f64) as i32;
        (*sb).s_blocks_per_group = 8 * SECTOR_SIZE as i32 * pow(2.0, (*sb).s_log_block_size as f64) as i32;
        
        if icount > (*sb).s_inodes_per_group as usize
        {
            (*sb).s_inodes_per_group = (*sb).s_inodes_per_group;
        }
        else {
            (*sb).s_inodes_per_group = icount as i32;
        }

        let time = sys_time();
        (*sb).s_mtime = (time.tick & 0xffffffff) as u32;
        (*sb).s_mtime_hi = ((time.tick >> 32) & 0xff) as u8;
        (*sb).s_wtime = (time.tick & 0xffffffff) as u32;
        (*sb).s_wtime_hi = ((time.tick >> 32) & 0xff) as u8;
        (*sb).s_max_mnt_count = -1;
        (*sb).s_magic = -4269;
        (*sb).s_state = 1;
        (*sb).s_errors = 1;
        (*sb).s_minor_rev_level = 0;
        (*sb).s_lastcheck = (time.tick & 0xffffffff) as u32;
        (*sb).s_lastcheck_hi = ((time.tick >> 32) & 0xff) as u8;
        (*sb).s_rev_level = 1;
        (*sb).s_first_ino = 11;
        (*sb).s_inode_size = 256;
        (*sb).s_feature_compat = 60;
        (*sb).s_feature_incompat = 706;
        (*sb).s_feature_ro_compat = 1131;

        let uuid = [ 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 ];
        (*sb).s_uuid = uuid;
        (*sb).s_reserved_gdt_blocks = 3;
        (*sb).s_journal_inum = 8;

        (*sb).s_def_hash_version = DEF_HASH_VERSION_HALF_MD4;
        (*sb).s_jnl_backup_type = 0; // no journal backup
        (*sb).s_desc_size = 64;
        (*sb).s_default_mount_opts = 12;
        (*sb).s_mkfs_time = time.tick as u32;
        (*sb).s_min_extra_isize = 32;
        (*sb).s_want_extra_isize = 32;
        (*sb).s_flags = 1; // signed directory hash

        (*sb).s_log_groups_per_flex = 4;
        (*sb).s_checksum_type = 1; // crc32c
        ext4_flax_group_init(dev, sb);


        (*buf).write_to_device(dev, 0, SECTOR_SIZE as usize * pow(2.0, (*sb).s_log_block_size as f64) as usize);
        (*buf).dispose();
    }
}

pub fn ext4_load_all_entries(dentry : &mut DEntry, inode : &mut Inode)
{
    unsafe
    {
        assert!(is_dir((*((*inode).inode_desc_ptr as *mut Ext4Inode)).i_mode));
        let sb = (*inode).logical_part_ptr;
        let dir_size = (*inode).get_size();
        let readable_buffer = alloc::alloc::alloc(Layout::from_size_align(dir_size, 1).unwrap()) as *mut c_void;
        FS.read_inode(inode, readable_buffer, dir_size, 0);
        let mut direntry_ptr = readable_buffer as *mut Ext4DirEntry2;
        let mut offset = 0;
        while dir_size > offset + (*direntry_ptr).rec_len as usize {
            let name = String::from_raw_parts((*direntry_ptr).name.as_ptr() as *mut u8, (*direntry_ptr).name_len as usize, (*direntry_ptr).name_len as usize);
            let inode = (*sb).iget((*direntry_ptr).inode as u64);
            direntry_ptr = (direntry_ptr as *mut c_void).offset((*direntry_ptr).rec_len as isize) as *mut Ext4DirEntry2;
            let child = dentry.new_child(&name);
            (*child).d_inode = inode;
            offset += (*direntry_ptr).rec_len as usize;
        }
        let name = String::from_raw_parts((*direntry_ptr).name.as_ptr() as *mut u8, (*direntry_ptr).name_len as usize, (*direntry_ptr).name_len as usize);
        let inode = (*sb).iget((*direntry_ptr).inode as u64);
        let child = dentry.new_child(&name);
        (*child).d_inode = inode;
        alloc::alloc::dealloc(readable_buffer as *mut u8, Layout::from_size_align((*inode).get_size(), 1).unwrap());
    }

}

pub fn ext4_find_entry(dir : &mut Inode, name : *const c_char, next : &mut *mut c_char, result_entry_ptr : &mut DirEntry)
{
    unsafe
    {
        assert!(is_dir((*((*dir).inode_desc_ptr as *mut Ext4Inode)).i_mode));
        let readable_buffer = alloc::alloc::alloc(Layout::from_size_align((*dir).get_size(), 1).unwrap()) as *mut c_void;
        let logical_part = &*dir.logical_part_ptr;
        let mut offset = 0;
        // let entries = (*dir_entry).count;
        let mut read_size = FS.read_inode(dir, readable_buffer, (*dir).get_size(), 0);
        let mut direntry_ptr = readable_buffer as *mut Ext4DirEntry2;
        while (*dir).get_size() > offset + (*direntry_ptr).rec_len as usize {
            let mut direntry = DirEntry::new(direntry_ptr as *mut c_void, crate::fs::file::FSType::Ext4);
            if direntry.match_name(name, next) && direntry.get_entry_point_to() != 0
            {
                let dir_entry_ptr = alloc::alloc::alloc(Layout::from_size_align(direntry.get_entry_ptr_size(), 1).unwrap()) as *mut c_void;
                compiler_builtins::mem::memcpy(dir_entry_ptr as *mut u8, direntry.entry_ptr as *const u8, direntry.get_entry_ptr_size());
                direntry.entry_ptr = dir_entry_ptr;
                *result_entry_ptr = direntry;
                break;
            }
            direntry.print_entry_name();
            direntry_ptr = (direntry_ptr as *mut c_void).offset((*direntry_ptr).rec_len as isize) as *mut Ext4DirEntry2;
            if direntry.get_entry_point_to() == 0
            {
                direntry.dir_entry_type = FSType::None;
                *result_entry_ptr = direntry
            }
        }
        alloc::alloc::dealloc(readable_buffer as *mut u8, Layout::from_size_align((*dir).get_size(), 1).unwrap());
    }
}

fn get_logic_block(logical_part : &mut LogicalPart, inode : *mut Inode, idx : Idx, create : bool, mut level : u8) -> Idx
{
    unsafe
    {
        let mut nr = idx;
        let mut dst_block = (&mut (*((*inode).inode_desc_ptr as *mut Ext4Inode)).i_block) as *mut i32;
        
        loop {
            if level == 0
            {
                return *dst_block.offset(nr as isize) as u64;
            }
            let buffer = logical_part.get_buffer(nr);
            (*buffer).read_from_device(logical_part.s_dev, *dst_block.offset(nr as isize) as Idx, logical_part.logic_block_size as usize * 1024);
            dst_block = (*buffer).buffer as *mut i32;
            nr = *dst_block.offset(nr as isize) as Idx;
            level -= 1;
        }
    }
}

#[inline(always)]
fn ext4_get_data_block_group_idx(logical_part : &LogicalPart, idx : Idx) -> usize
{
    unsafe
    {
        idx as usize / (*(logical_part.s_sbi as *const Ext4SuperBlockInfo)).blocks_per_group
    }
}

pub fn ext4_get_logic_block_idx(logical_part : &mut LogicalPart, inode : *mut Inode, idx : Idx, create : bool) -> Idx
{
    unsafe
    {
        let ext4_inode = (*inode).inode_desc_ptr as *mut Ext4Inode;
        let block_desc_node = &mut (*ext4_inode).i_block as *mut i32 as *mut Ext4InodeExtentDesc;
        assert!((*block_desc_node).head.eh_magic as u16 == 0xf30a);
        loop {
            if (*block_desc_node).head.eh_depth == 0
            {
                let mut var = 0;
                while var < 4 {
                    if (*block_desc_node).node[var].leaf_node.ee_block as u64 <= idx && ((*block_desc_node).node[var].leaf_node.ee_block as u64 + (*block_desc_node).node[var].leaf_node.ee_len as u64) > idx
                    {
                        return (*block_desc_node).node[var].leaf_node.ee_start_lo as Idx + (((*block_desc_node).node[var].leaf_node.ee_start_hi as Idx) << 32) + idx - (*block_desc_node).node[var].leaf_node.ee_block as Idx;
                    }
                    var += 1;
                }
                panic!("read file block out of range!\n");
            }
            else {
                let mut buff = null_mut();
                let mut var = 0;
                let mut extent_block = null_mut();
                while var < ((*block_desc_node).head.eh_entries - 1) as usize {
                    if (*block_desc_node).node[var + 1].nonleaf.ei_block as Idx > idx
                    {
                        let dst_block = (((*block_desc_node).node[var].nonleaf.ei_leaf_hi as Idx) << 32) + ((*block_desc_node).node[var].nonleaf.ei_leaf_lo as Idx);
                        buff = logical_part.read_block(dst_block as usize);
                        extent_block = (*buff).buffer as *mut Ext4ExtentBlock;
                        break;
                    }
                    var += 1;
                }
                while !extent_block.is_null() && (*extent_block).head.eh_depth != 0 {
                    var = 0;
                    while ((*extent_block).node[var].nonleaf.ei_block as Idx) < idx {
                        let dst_block = (((*extent_block).node[var].nonleaf.ei_leaf_hi as Idx) << 32) + ((*extent_block).node[var].nonleaf.ei_leaf_lo as Idx);
                        logical_part.release_buffer(buff, idx);
                        buff = logical_part.read_block(dst_block as usize);
                        extent_block = (*buff).buffer as *mut Ext4ExtentBlock;
                    }
                }
                while ((*extent_block).node[var].leaf_node.ee_block as Idx + (*extent_block).node[var].leaf_node.ee_len as Idx) > idx {
                    logical_part.release_buffer(buff, idx);
                    return (*extent_block).node[var].leaf_node.ee_start_lo as Idx + (((*extent_block).node[var].leaf_node.ee_start_hi as Idx) << 32) + idx - (*extent_block).node[var].leaf_node.ee_block as Idx;
                }
            }
        }
    }
}

pub fn ext2_or_ext3_get_logic_block_idx(logical_part : &mut LogicalPart, inode : *mut Inode, idx : Idx, create : bool) -> Idx
{
    let mut level = 0;
    if (idx as usize) < EXT4_IND_BLOCK
    {
        return get_logic_block(logical_part, inode, idx, create, level);
    }
    if idx < EXT4_INDIRECT1_BLOCK
    {
        level = 1;
        return get_logic_block(logical_part, inode, EXT4_DIND_BLOCK as u64 - 1, create, level);
    }
    if idx < EXT4_INDIRECT2_BLOCK
    {
        level = 2;
        return  get_logic_block(logical_part, inode, EXT4_TIND_BLOCK as u64 - 1, create, level);
    }
    level = 3;
    get_logic_block(logical_part, inode, EXT4_N_BLOCKS as u64 - 1, create, level)
}

pub fn ext4_inode_block_read(logical_part : *mut LogicalPart, inode : *mut Inode, mut block_idx : Idx) -> *mut Buffer
{
    unsafe
    {
        block_idx = (*logical_part).get_logic_block_idx(inode, block_idx, false);
        let buffer = (*logical_part).get_buffer(block_idx);
        if !(*buffer).is_avaliable()
        {
            (*buffer).read_from_device((*logical_part).s_dev, block_idx * 2 * (*logical_part).logic_block_size as Idx, 2 * (*logical_part).logic_block_size as usize);
        }
        buffer
    }
}

fn ext4_has_metadata_csum(super_block : *const Ext4SuperBlock) -> bool
{
    unsafe
    {
        ((*super_block).s_feature_ro_compat & RO_COMPAT_METADATA_CSUM) != 0
    }
}


pub fn ext4_load_block_bitmap(gbi : &mut Ext4GroupDescInfo)
{
    unsafe
    {
        let grop_desc = gbi.group_desc_ptr as *const Ext4GroupDesc;
        let idx = (((*grop_desc).bg_block_bitmap_hi as usize) << 32) | (*grop_desc).bg_block_bitmap_lo as usize;
        let block_map_buffer = (*gbi.parent).read_block(idx);
        let block_map = alloc::alloc::alloc(Layout::new::<[c_void; PAGE_SIZE]>());
        let raw_super_block = (*((*gbi.parent).s_sbi as *mut Ext4SuperBlockInfo)).super_block as *mut Ext4SuperBlock;
        let size = (*raw_super_block).s_clusters_per_group / 8;
        (*block_map_buffer).read_from_buffer(block_map as *mut c_void, 0, 1024 * (*gbi.parent).logic_block_size as usize);
        let calculated = crc32c_le((*((*gbi.parent).s_sbi as *mut Ext4SuperBlockInfo)).s_csum_seed, block_map as *const c_void, size as usize);
        let provided = ((*grop_desc).bg_block_bitmap_csum_lo as u32) | (((*grop_desc).bg_block_bitmap_csum_hi as u32) << 16);
        if provided != calculated
        {
            panic!("checksum error in loading block bitmap");
        }
        gbi.logical_block_bitmap.reset_bitmap(block_map, (*raw_super_block).s_blocks_per_group as usize);
    }
}

pub fn ext4_load_inode_bitmaps(gbi : &mut Ext4GroupDescInfo)
{
    unsafe
    {
        let grop_desc = gbi.group_desc_ptr as *const Ext4GroupDesc;
        let idx = (((*grop_desc).bg_inode_bitmap_hi as usize) << 32) | (*grop_desc).bg_inode_bitmap_lo as usize;
        let inode_map_buffer = (*gbi.parent).read_block(idx);
        let inode_map = alloc::alloc::alloc(Layout::new::<[c_void; PAGE_SIZE]>());
        let raw_super_block = (*((*gbi.parent).s_sbi as *mut Ext4SuperBlockInfo)).super_block as *mut Ext4SuperBlock;
        let size = (*raw_super_block).s_inodes_per_group / 8;
        (*inode_map_buffer).read_from_buffer(inode_map as *mut c_void, 0, 1024 * (*gbi.parent).logic_block_size as usize);
        let calculated = crc32c_le((*((*gbi.parent).s_sbi as *const Ext4SuperBlockInfo)).s_csum_seed, inode_map as *const c_void, size as usize);
        let provided = ((*grop_desc).bg_inode_bitmap_csum_lo as u32) | (((*grop_desc).bg_inode_bitmap_csum_hi as u32) << 16);
        if provided != calculated
        {
            panic!("checksum error in loading block bitmap");
        }
        gbi.logical_block_bitmap.reset_bitmap(inode_map, (*raw_super_block).s_inodes_per_group as usize);
    }
}


pub fn ext4_group_desc_csum(logic_part : &LogicalPart, block_group : u32, gdp : *const Ext4GroupDesc) -> u16
{
    unsafe
    {
        let mut group_desc_checksum;
        let mut offset = offset_of!(Ext4GroupDesc, bg_checksum);
        let raw_super_block = (*(logic_part.s_sbi as *mut Ext4SuperBlockInfo)).super_block as *mut Ext4SuperBlock;
        if ext4_has_metadata_csum(raw_super_block)
        {
            let dummy_csum = 0u16;
            let mut crc32 = crc32c_le((*(logic_part.s_sbi as *mut Ext4SuperBlockInfo)).s_csum_seed, &block_group as *const u32 as *const c_void, 4);
            crc32 = crc32c_le(crc32, gdp as *const c_void, offset);
            crc32 = crc32c_le(crc32, &dummy_csum as *const u16 as  *const c_void, 2);
            offset += 2;
            crc32 = crc32c_le(crc32, (gdp as *const c_void).offset(offset as isize), size_of::<Ext4GroupDesc>() - offset);
            return (crc32 & 0xffff) as u16;
        }
        group_desc_checksum = crc16(!0, (*raw_super_block).s_uuid.as_ptr() as *const c_void, 16);
        group_desc_checksum = crc16(group_desc_checksum, &block_group as *const u32 as *const c_void, 4);
        group_desc_checksum = crc16(group_desc_checksum, gdp as *const c_void, offset_of!(Ext4GroupDesc, bg_checksum));
        group_desc_checksum
    }

}

pub fn ext4_inode_read(logical_part : &mut LogicalPart, inode : *mut Inode, mut dst : *mut c_void, len : usize, offset : usize) -> i64
{
    unsafe
    {
        let ext4_desc_ptr = (*inode).inode_desc_ptr as *mut Ext4Inode;
        assert!(is_file((*ext4_desc_ptr).i_mode) || is_dir((*ext4_desc_ptr).i_mode));
        let file_size = ((*ext4_desc_ptr).i_size_lo as i64 + (((*ext4_desc_ptr).i_size_high as i64) << 32)) as usize;
        if offset >= file_size
        {
            return EOF;
        }
        let mut read_begin = offset;
        let mut left = min(len, file_size - offset);
        while left > 0 {
            let idx = offset as u64 / 1024 / logical_part.logic_block_size as u64;
            let buffer = ext4_inode_block_read(logical_part, inode, idx);
            let start = read_begin % (1024 * logical_part.logic_block_size as usize);
            let read_num = min((1024 * logical_part.logic_block_size as usize) - start, left);
            left -= read_num;
            read_begin += read_num;
            (*buffer).read_from_buffer(dst, start, read_num);
            dst = dst.offset(read_num as isize);
            logical_part.release_buffer(buffer, idx);
        }
        return (read_begin - offset) as i64;
    }
}

#[inline(always)]
pub fn is_file(f_mode : u16) -> bool
{
    f_mode & Ext4FileMode::IFMT.bits() == Ext4FileMode::IFREG.bits()
}

#[inline(always)]
pub fn is_dir(f_mode : u16) -> bool
{
    f_mode & Ext4FileMode::IFMT.bits() == Ext4FileMode::IFDIR.bits()
}

#[inline(always)]
pub fn is_chr(f_mode : u16) -> bool
{
    f_mode & Ext4FileMode::IFMT.bits() == Ext4FileMode::IFCHR.bits()
}

#[inline(always)]
pub fn is_blk(f_mode : u16) -> bool
{
    f_mode & Ext4FileMode::IFMT.bits() == Ext4FileMode::IFBLK.bits()
}

#[inline(always)]
pub fn is_fifo(f_mode : u16) -> bool
{
    f_mode & Ext4FileMode::IFMT.bits() == Ext4FileMode::IFIFO.bits()
}

#[inline(always)]
pub fn is_lnk(f_mode : u16) -> bool
{
    f_mode & Ext4FileMode::IFMT.bits() == Ext4FileMode::IFLNK.bits()
}

#[inline(always)]
pub fn is_sock(f_mode : u16) -> bool
{
    f_mode & Ext4FileMode::IFMT.bits() == Ext4FileMode::IFSOCK.bits()
}

#[inline(always)]
pub fn is_reg(f_mode : u16) -> bool
{
    f_mode & Ext4FileMode::IFMT.bits() == Ext4FileMode::IFREG.bits()
}

fn to_file_mode(file_mode : &Ext4FileMode) -> FileMode
{
    FileMode::from_bits(file_mode.bits()).unwrap()
}

pub fn ext4_iget(logical_part : &mut LogicalPart, inode : *mut Inode, logic_block_size : i32, nr : Idx)
{
    unsafe {
        let block_no = ext4_get_inode_logical_block(logical_part, nr);
        let buffer = logical_part.read_block(block_no as usize);
        (*inode).inode_block_buffer = buffer;
        (*inode).inode_desc_ptr = (*buffer).buffer.offset((256 * (nr - 1) % (1024 * logic_block_size as u64)).try_into().unwrap());
        let raw_inode = (*buffer).buffer.offset((256 * (nr - 1) % (1024 * logic_block_size as u64)).try_into().unwrap()) as *const Ext4Inode;
        (*inode).i_mode = to_file_mode(&Ext4FileMode::from_bits((*raw_inode).i_mode & Ext4FileMode::IFMT.bits()).unwrap());
        (*inode).i_perm = FSPermission::from_bits((*raw_inode).i_mode & FSPermission::MASK.bits()).unwrap();
        (*inode).i_uid = (*raw_inode).i_uid as u32;
        (*inode).i_gid = (*raw_inode).i_gid as u32;
        (*inode).i_nlink = AtomicI64::new((*raw_inode).i_links_count as i64);
        // special inode
        if (*inode).is_blk() || (*inode).is_chr() || (*inode).is_fifo() || (*inode).is_sock()
        {
            if (*raw_inode).i_block[0] != 0
            {
                (*inode).i_rdev = old_decode_dev((*raw_inode).i_block[0] as DevT);
            }
            else {
                (*inode).i_rdev = new_decode_dev((*raw_inode).i_block[1] as DevT);
                
            }
        }
    }
}

#[repr(C)]
pub struct PartEntry
{
    pub bootable : u8,             // 引导标志
    pub start_head : u8,           // 分区起始磁头号
    pub tart_sector : u8,     // 分区起始扇区号
    pub start_cylinder : u8, // 分区起始柱面号
    pub system : u8,               // 分区类型字节
    pub end_head : u8,             // 分区的结束磁头号
    pub end_sector : u8,       // 分区结束扇区号
    pub end_cylinder : u8,   // 分区结束柱面号
    pub start : u32,               // 分区起始物理扇区号 LBA
    pub count : u32,               // 分区占用的扇区数
}

#[repr(C)]
pub union Ext4ExtentDescTreeNode {
    leaf_node : core::mem::ManuallyDrop<Ext4Extent>,
    nonleaf : core::mem::ManuallyDrop<Ext4ExtentIdx>
}

#[repr(C)]
pub struct Ext4InodeExtentDesc
{
    head : Ext4ExtentHeader,
    node : [Ext4ExtentDescTreeNode; 4]
}

#[repr(C)]
pub struct Ext4ExtentBlock
{
    head : Ext4ExtentHeader,
    node : [Ext4ExtentDescTreeNode; 340],
    tail : Ext4ExtentTail
}

pub struct Ext4ExtentTail
{
    et_check_sum : u32
}

#[repr(C)]
pub struct Ext4ExtentHeader
{
    eh_magic : i16,
    eh_entries : i16,
    eh_max : i16,
    eh_depth : i16,
    eh_generation : i32
}

#[repr(C)]
pub struct Ext4DirEntry2
{  
    pub inode : i32,          /* Inode number */  
    pub rec_len : i16,         /* Directory entry length */  
    pub name_len : u8,       /* Name length */  
    pub file_type : u8,  
    pub name : [c_char; EXT4_NAME_LEN]    /* File name */  
}

#[repr(C)]
pub struct Ext4DirEntry
{  
    pub inode : i32,          /* Inode number */  
    pub rec_len  : i16,        /* Directory entry length */  
    pub name_len : i16,       /* Name length */  
    pub name : [c_char; EXT4_NAME_LEN]    /* File name */  
}

#[repr(C)]
pub struct Ext4GroupDesc  
{  
    pub bg_block_bitmap_lo : u32, /* Blocks bitmap block */  
    pub bg_inode_bitmap_lo : u32, /* Inodes bitmap block */  
    pub bg_inode_table_lo : i32,  /* Inodes table block */  
    pub bg_free_blocks_count_lo : u16,/* Free blocks count */  
    pub bg_free_inodes_count_lo : u16,/* Free inodes count */  
    pub bg_used_dirs_count_lo : i16,  /* Directories count */  
    pub bg_flags : i16,       /* EXT4_BG_flags (INODE_UNINIT, etc) */  
    pub bg_exclude_bitmap_lo : i32,   /* Exclude bitmap for snapshots */  
    pub bg_block_bitmap_csum_lo : u16,/* crc32c(s_uuid+grp_num+bbitmap) LE */  
    pub bg_inode_bitmap_csum_lo : u16,/* crc32c(s_uuid+grp_num+ibitmap) LE */  
    pub bg_itable_unused_lo : i16,    /* Unused inodes count */  
    pub bg_checksum : u16,        /* crc16(sb_uuid+group+desc) */  
    pub bg_block_bitmap_hi : i32, /* Blocks bitmap block MSB */  
    pub bg_inode_bitmap_hi : i32, /* Inodes bitmap block MSB */  
    pub bg_inode_table_hi : i32,  /* Inodes table block MSB */  
    pub bg_free_blocks_count_hi : i16,/* Free blocks count MSB */  
    pub bg_free_inodes_count_hi : i16,/* Free inodes count MSB */  
    pub bg_used_dirs_count_hi : i16,  /* Directories count MSB */  
    pub bg_itable_unused_hi : i16,    /* Unused inodes count MSB */  
    pub bg_exclude_bitmap_hi : i32,   /* Exclude bitmap block MSB */  
    pub bg_block_bitmap_csum_hi : u16,/* crc32c(s_uuid+grp_num+bbitmap) BE */  
    pub bg_inode_bitmap_csum_hi : u16,/* crc32c(s_uuid+grp_num+ibitmap) BE */  
    pub bg_reserved : u32  
}

#[repr(C)]
pub struct Ext4ExtentIdx
{  
    ei_block : i32,   /* index covers logical blocks from 'block' */  
    ei_leaf_lo : i32, /* pointer to the physical block of the next * 
    * level. leaf or next index could be there */  
    ei_leaf_hi : i16, /* high 16 bits of physical block */  
    ei_unused : i16  
}

#[repr(C)]
pub struct Ext4Inode
{
    pub i_mode : u16,
    pub i_uid : i16,
    pub i_size_lo : i32,
    pub i_atime : i32,
    pub i_ctime : i32,
    pub i_mtime : i32,
    pub i_dtime : i32,
    pub i_gid : i16,
    pub i_links_count : i16,
    pub i_blocks_lo : i32,
    pub i_flags : i32,
    pub osd1 : Osd1,
    pub i_block : [i32; EXT4_N_BLOCKS],
    pub i_generation : i32,
    pub i_file_acl_lo : i32,
    pub i_size_high : i32,
    pub i_obso_faddr : i32,
    pub osd2 : Osd2,
    pub i_extra_isize : i16,
    pub i_checksum_hi : i16,
    pub i_ctime_extra : i32,
    pub i_mtime_extra : i32,
    pub i_atime_extra : i32,
    pub i_crtime : i32,
    pub i_crtime_extra : i32,
    pub i_version_hi : i32,
    pub i_projid : i32,
}

#[repr(C)]
pub struct Linux2
{
    l_i_blocks_high : i16,
    l_i_file_acl_high : i16,
    l_i_uid_high : i16,
    l_i_gid_high : i16,
    l_i_check_sum_lo : i16,
    l_i_reserved : i16
}

#[repr(C)]
pub struct Hurd2
{
    h_i_reserved1 : i16,
    h_i_mode_high : u16,
    h_i_uid_high : u16,
    h_i_gid_high : u16,
    h_i_author : u16
}

#[repr(C)]
pub struct Masix2
{
    h_i_reserved1 : i16,
    m_i_file_acl_high : i16,
    m_i_reserced2 : [u32; 2]
}

#[repr(C)]
pub union Osd2
{
    linux2 : core::mem::ManuallyDrop<Linux2>,
    hurd2 : core::mem::ManuallyDrop<Hurd2>,
    masix2 : core::mem::ManuallyDrop<Masix2>
}

#[repr(C)]
pub union Osd1 {
    pub l_i_version : i32,
    pub h_itranslator : u32,
    pub m_ireserved1 : u32,
}

#[repr(C, packed)]
pub struct Ext4SuperBlock
{
    pub s_inodes_count : u32,		/* Inodes count */
    pub s_blocks_count_lo : u32,	/* Blocks count */
    pub s_r_blocks_count_lo : u32,	/* Reserved blocks count */
    pub s_free_blocks_count_lo : u32,	/* Free blocks count */
    pub s_free_inodes_count : i32,	/* Free inodes count */
    pub s_first_data_block : i32,	/* First Data Block */
    pub s_log_block_size : i32,	/* Block size */
    pub s_log_cluster_size : i32,	/* Allocation cluster size */
    pub s_blocks_per_group : i32,	/* # Blocks per group */
    pub s_clusters_per_group : i32,	/* # Clusters per group */
    pub s_inodes_per_group : i32,	/* # Inodes per group */
    pub s_mtime : u32,		/* Mount time */
    pub s_wtime : u32,		/* Write time */
    pub s_mnt_count : i16,		/* Mount count */
    pub s_max_mnt_count : i16,	/* Maximal mount count */
    pub s_magic : i16,		/* Magic signature */
    pub s_state : i16,		/* File system state */
    pub s_errors : i16,		/* Behaviour when detecting errors */
    pub s_minor_rev_level : i16,	/* minor revision level */
    pub s_lastcheck : u32,		/* time of last check */
    pub s_checkinterval : i32,	/* max. time between checks */
    pub s_creator_os : i32,		/* OS */
    pub s_rev_level : i32,		/* Revision level */
    pub s_def_resuid : i16,		/* Default uid for reserved blocks */
    pub s_def_resgid : i16,		/* Default gid for reserved blocks */
        /*
         * These fields are for EXT4_DYNAMIC_REV superblocks only.
         *
         * Note: the difference between the compatible feature set and
         * the incompatible feature set is that if there is a bit set
         * in the incompatible feature set that the kernel doesn't
         * know about, it should refuse to mount the filesystem.
         *
         * e2fsck's requirements are more strict; if it doesn't know
         * about a feature in either the compatible or incompatible
         * feature set, it must abort and not try to meddle with
         * things it doesn't understand...
         */
    pub s_first_ino : i32,		/* First non-reserved inode */
    pub s_inode_size : i16,		/* size of inode structure */
    pub s_block_group_nr : i16,	/* block group # of this superblock */
    pub s_feature_compat : i32,	/* compatible feature set */
    pub s_feature_incompat : i32,	/* incompatible feature set */
    pub s_feature_ro_compat : i32,	/* readonly-compatible feature set */
    pub s_uuid : [u8; 16],		/* 128-bit uuid for volume */
    pub s_volume_name : [c_char; EXT4_LABEL_MAX],	/* volume name */
    pub s_last_mounted : [c_char; 64],	/* directory where last mounted */
    pub s_algorithm_usage_bitmap : i32, /* For compression */
        /*
         * Performance hints.  Directory preallocation should only
         * happen if the EXT4_FEATURE_COMPAT_DIR_PREALLOC flag is on.
         */
    pub s_prealloc_blocks : u8,	/* Nr of blocks to try to preallocate*/
    pub s_prealloc_dir_blocks : u8,	/* Nr to preallocate for dirs */
    pub s_reserved_gdt_blocks : i16,	/* Per group desc for online growth */
        /*
         * Journaling support valid if EXT4_FEATURE_COMPAT_HAS_JOURNAL set.
         */
    pub s_journal_uuid : [u8; 16],	/* uuid of journal superblock */
    pub s_journal_inum : i32,		/* inode number of journal file */
    pub s_journal_dev : i32,		/* device number of journal file */
    pub s_last_orphan : i32,		/* start of list of inodes to delete */
    pub s_hash_seed : [i32; 4],		/* HTREE hash seed */
    pub s_def_hash_version : u8,	/* Default hash version to use */
    pub s_jnl_backup_type : u8,
    pub s_desc_size : i16,		/* size of group descriptor */
    pub s_default_mount_opts : i32,
    pub s_first_meta_bg : i32,	/* First metablock block group */
    pub s_mkfs_time : u32,		/* When the filesystem was created */
    pub s_jnl_blocks : [i32; 17],	/* Backup of the journal inode */
        /* 64bit support valid if EXT4_FEATURE_INCOMPAT_64BIT */
    pub s_blocks_count_hi : u32,	/* Blocks count */
    pub s_r_blocks_count_hi : u32,	/* Reserved blocks count */
    pub s_free_blocks_count_hi : u32,	/* Free blocks count */
    pub s_min_extra_isize : i16,	/* All inodes have at least # bytes */
    pub s_want_extra_isize : i16, 	/* New inodes should reserve # bytes */
    pub s_flags : i32,		/* Miscellaneous flags */
    pub s_raid_stride : i16,		/* RAID stride */
    pub s_mmp_update_interval : i16,  /* # seconds to wait in MMP checking */
    pub s_mmp_block : i64,            /* Block for multi-mount protection */
    pub s_raid_stripe_width : i32,    /* blocks on all data disks (N*stride)*/
    pub s_log_groups_per_flex : u8,  /* FLEX_BG group size */
    pub s_checksum_type : u8,	/* metadata checksum algorithm used */
    pub s_encryption_level : u8,	/* versioning level for encryption */
    pub s_reserved_pad : u8,		/* Padding to next 32bits */
    pub s_kbytes_written : i64,	/* nr of lifetime kilobytes written */
    pub s_snapshot_inum : i32,	/* Inode number of active snapshot */
    pub s_snapshot_id : i32,		/* sequential ID of active snapshot */
    pub s_snapshot_r_blocks_count : i64, /* reserved blocks for active
                              snapshot's future use */
    pub s_snapshot_list : i32,	/* inode number of the head of the
                           on-disk snapshot list */
    // #define EXT4_S_ERR_START offsetof(struct ext4_super_block, s_error_count)
    pub s_error_count : i32,		/* number of fs errors */
    pub s_first_error_time : i32,	/* first time an error happened */
    pub s_first_error_ino : i32,	/* inode involved in first error */
    pub s_first_error_block : i64,	/* block involved of first error */
    pub s_first_error_func : [u8; 32],	/* function where the error happened */
    pub s_first_error_line : i32,	/* line number where error happened */
    pub s_last_error_time : i32,	/* most recent time of an error */
    pub s_last_error_ino : i32,	/* inode involved in last error */
    pub s_last_error_line : i32,	/* line number where error happened */
    pub s_last_error_block : i64,	/* block involved of last error */
    pub s_last_error_func : [u8; 32],	/* function where the error happened */
    // #define EXT4_S_ERR_END offsetof(struct ext4_super_block, s_mount_opts)
    pub s_mount_opts : [u8; 64],
    pub s_usr_quota_inum : i32,	/* inode for tracking user quota */
    pub s_grp_quota_inum : i32,	/* inode for tracking group quota */
    pub s_overhead_clusters : i32,	/* overhead blocks/clusters in fs */
    pub s_backup_bgs : [i32; 2],	/* groups with sparse_super2 SBs */
    pub s_encrypt_algos : [u8; 4],	/* Encryption algorithms in use  */
    pub s_encrypt_pw_salt : [u8; 16],	/* Salt used for string2key algorithm */
    pub s_lpf_ino : i32,		/* Location of the lost+found inode */
    pub s_prj_quota_inum : i32,	/* inode for tracking project quota */
    pub s_checksum_seed : i32,	/* crc32c(uuid) if csum_seed set */
    pub s_wtime_hi : u8,
    pub s_mtime_hi : u8,
    pub s_mkfs_time_hi : u8,
    pub s_lastcheck_hi : u8,
    pub s_first_error_time_hi : u8,
    pub s_last_error_time_hi : u8,
    pub s_first_error_errcode : u8,
    pub s_last_error_errcode : u8,
    pub s_encoding : i16,		/* Filename charset encoding */
    pub s_encoding_flags : i16,	/* Filename charset encoding flags */
    pub s_orphan_file_inum : i32,	/* Inode for tracking orphan inodes */
    pub s_reserved : [i32; 94],		/* Padding to the end of the block */
    pub s_checksum : i32		/* crc32c(superblock) */
}

#[repr(C)]
pub struct Ext4Extent {
	pub ee_block : i32,	/* exient叶子的第一个数据块号 */
	pub ee_len : i16,		/* exient叶子的数据块数量 */
	pub ee_start_hi : i16,	/* 物理数据块的高16位 */
	pub ee_start_lo : i32,	/* 物理数据块的低32位 */
}

