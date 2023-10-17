use core::{ffi::{c_char, c_void}, cmp::min};

use crate::{fs::file::EOF, kernel::buffer::Buffer};

use super::file::{LogicalPart, Inode};



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
            (*buffer).read_from_device(logical_part.dev, *dst_block.offset(nr as isize) as Idx, logical_part.logic_block_size as usize * 1024);
            dst_block = (*buffer).buffer as *mut i32;
            nr = *dst_block.offset(nr as isize) as Idx;
            level -= 1;
        }
    }
}

pub fn ext4_get_logic_block(logical_part : &mut LogicalPart, inode : *mut Inode, idx : Idx, create : bool) -> Idx
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

pub fn ext4_inode_read(logical_part : &mut LogicalPart, inode : *mut Inode, mut dst : *mut c_void, len : usize, offset : usize) -> i64
{
    unsafe
    {
        let ext4_desc_ptr = (*inode).inode_desc_ptr as *mut Ext4Inode;
        assert!(is_file((*ext4_desc_ptr).i_mode) || is_dir((*ext4_desc_ptr).i_mode));
        let read_num = ((*ext4_desc_ptr).i_size_lo as i64 + (((*ext4_desc_ptr).i_size_high as i64) << 32)) as usize;
        if offset >= read_num
        {
            return EOF;
        }
        let mut read_begin = offset;
        let mut left = min(len, read_num - offset);
        while left > 0 {
            let idx = offset as u64 / 1024 / logical_part.logic_block_size as u64;
            let buffer = logical_part.get_buffer(idx);
            if !(*buffer).is_avaliable()
            {
                let idx = logical_part.get_logic_block(inode, idx, false);
                (*buffer).read_from_device(logical_part.dev, idx, 2 * logical_part.logic_block_size as usize);
            }
            let start = read_begin % (1024 * logical_part.logic_block_size as usize);
            let read_num = min((1024 * logical_part.logic_block_size as usize) - start, left);
            left -= read_num;
            read_begin += read_num;
            (*buffer).read_from_buffer(dst, start, read_num);
            dst = dst.offset(read_num as isize);
            logical_part.release_buffer(buffer);
        }
        return (read_begin - offset) as i64;
    }
}

#[inline]
pub fn is_file(f_mode : u16) -> bool
{
    f_mode & Ext4FileMode::IFMT.bits() == Ext4FileMode::IFREG.bits()
}

#[inline]
pub fn is_dir(f_mode : u16) -> bool
{
    f_mode & Ext4FileMode::IFMT.bits() == Ext4FileMode::IFDIR.bits()
}

#[inline]
pub fn is_chr(f_mode : u16) -> bool
{
    f_mode & Ext4FileMode::IFMT.bits() == Ext4FileMode::IFCHR.bits()
}

#[inline]
pub fn is_blk(f_mode : u16) -> bool
{
    f_mode & Ext4FileMode::IFMT.bits() == Ext4FileMode::IFBLK.bits()
}

#[inline]
pub fn is_fifo(f_mode : u16) -> bool
{
    f_mode & Ext4FileMode::IFMT.bits() == Ext4FileMode::IFIFO.bits()
}

#[inline]
pub fn is_lnk(f_mode : u16) -> bool
{
    f_mode & Ext4FileMode::IFMT.bits() == Ext4FileMode::IFLNK.bits()
}

#[inline]
pub fn is_sock(f_mode : u16) -> bool
{
    f_mode & Ext4FileMode::IFMT.bits() == Ext4FileMode::IFSOCK.bits()
}

#[inline]
pub fn is_reg(f_mode : u16) -> bool
{
    f_mode & Ext4FileMode::IFMT.bits() == Ext4FileMode::IFREG.bits()
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
pub struct Ext4ExtentHeader
{
    eh_magic : i16,
    eh_entries : i16,
    eh_max : i16,
    eh_depth : i16,
    eh_generation : i32
}

struct Ext4DirEntry2
{  
    inode : i32,          /* Inode number */  
    rec_len : i16,         /* Directory entry length */  
    name_len : u8,       /* Name length */  
    file_type : u8,  
    name : [c_char; EXT4_NAME_LEN]    /* File name */  
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
    pub bg_block_bitmap_lo : i32, /* Blocks bitmap block */  
    pub bg_inode_bitmap_lo : i32, /* Inodes bitmap block */  
    pub bg_inode_table_lo : i32,  /* Inodes table block */  
    pub bg_free_blocks_count_lo : i16,/* Free blocks count */  
    pub bg_free_inodes_count_lo : i16,/* Free inodes count */  
    pub bg_used_dirs_count_lo : i16,  /* Directories count */  
    pub bg_flags : i16,       /* EXT4_BG_flags (INODE_UNINIT, etc) */  
    pub bg_exclude_bitmap_lo : i32,   /* Exclude bitmap for snapshots */  
    pub bg_block_bitmap_csum_lo : i16,/* crc32c(s_uuid+grp_num+bbitmap) LE */  
    pub bg_inode_bitmap_csum_lo : i16,/* crc32c(s_uuid+grp_num+ibitmap) LE */  
    pub bg_itable_unused_lo : i16,    /* Unused inodes count */  
    pub bg_checksum : i16,        /* crc16(sb_uuid+group+desc) */  
    pub bg_block_bitmap_hi : i32, /* Blocks bitmap block MSB */  
    pub bg_inode_bitmap_hi : i32, /* Inodes bitmap block MSB */  
    pub bg_inode_table_hi : i32,  /* Inodes table block MSB */  
    pub bg_free_blocks_count_hi : i16,/* Free blocks count MSB */  
    pub bg_free_inodes_count_hi : i16,/* Free inodes count MSB */  
    pub bg_used_dirs_count_hi : i16,  /* Directories count MSB */  
    pub bg_itable_unused_hi : i16,    /* Unused inodes count MSB */  
    pub bg_exclude_bitmap_hi : i32,   /* Exclude bitmap block MSB */  
    pub bg_block_bitmap_csum_hi : i16,/* crc32c(s_uuid+grp_num+bbitmap) BE */  
    pub bg_inode_bitmap_csum_hi : i16,/* crc32c(s_uuid+grp_num+ibitmap) BE */  
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
    pub i_links_coint : i16,
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

#[repr(C)]
pub struct Ext4SuperBlock
{
    pub s_inodes_count : i32,		/* Inodes count */
    pub s_blocks_count_lo : i32,	/* Blocks count */
    pub s_r_blocks_count_lo : i32,	/* Reserved blocks count */
    pub s_free_blocks_count_lo : i32,	/* Free blocks count */
    pub s_free_inodes_count : i32,	/* Free inodes count */
    pub s_first_data_block : i32,	/* First Data Block */
    pub s_log_block_size : i32,	/* Block size */
    pub s_log_cluster_size : i32,	/* Allocation cluster size */
    pub s_blocks_per_group : i32,	/* # Blocks per group */
    pub s_clusters_per_group : i32,	/* # Clusters per group */
    pub s_inodes_per_group : i32,	/* # Inodes per group */
    pub s_mtime : i32,		/* Mount time */
    pub s_wtime : i32,		/* Write time */
    pub s_mnt_count : i16,		/* Mount count */
    pub s_max_mnt_count : i16,	/* Maximal mount count */
    pub s_magic : i16,		/* Magic signature */
    pub s_state : i16,		/* File system state */
    pub s_errors : i16,		/* Behaviour when detecting errors */
    pub s_minor_rev_level : i16,	/* minor revision level */
    pub s_lastcheck : i32,		/* time of last check */
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
    pub s_mkfs_time : i32,		/* When the filesystem was created */
    pub s_jnl_blocks : [i32; 17],	/* Backup of the journal inode */
        /* 64bit support valid if EXT4_FEATURE_INCOMPAT_64BIT */
    pub s_blocks_count_hi : i32,	/* Blocks count */
    pub s_r_blocks_count_hi : i32,	/* Reserved blocks count */
    pub s_free_blocks_count_hi : i32,	/* Free blocks count */
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
    pub s_first_error_block : i16,	/* block involved of first error */
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

pub struct Ext4Extent {
	pub ee_block : i32,	/* exient叶子的第一个数据块号 */
	pub ee_len : i16,		/* exient叶子的数据块数量 */
	pub ee_start_hi : i16,	/* 物理数据块的高16位 */
	pub ee_start_lo : i32,	/* 物理数据块的低32位 */
}

