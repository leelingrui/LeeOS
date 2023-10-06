use core::ffi::{c_char, c_void};
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

pub type Idx = u64; 

pub struct FileSystem
{
    disk_number : usize,
    super_block : *mut Ext4SuperBlock
}

impl FileSystem {
    pub fn load_super_block(super_block : *mut c_void)
    {
        unsafe
        {
            let sb = super_block as *mut Ext4SuperBlock;
            if (*sb).s_magic == 0
            {
    
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
struct Ext4ExtentHeader
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
struct Ext4DirEntry
{  
    inode : i32,          /* Inode number */  
    rec_len  : i16,        /* Directory entry length */  
    name_len : i16,       /* Name length */  
    name : [c_char; EXT4_NAME_LEN]    /* File name */  
}

#[repr(C)]
struct Ext4GroupDesc  
{  
    bg_block_bitmap_lo : i32, /* Blocks bitmap block */  
    bg_inode_bitmap_lo : i32, /* Inodes bitmap block */  
    bg_inode_table_lo : i32,  /* Inodes table block */  
    bg_free_blocks_count_lo : i16,/* Free blocks count */  
    bg_free_inodes_count_lo : i16,/* Free inodes count */  
    bg_used_dirs_count_lo : i16,  /* Directories count */  
    bg_flags : i16,       /* EXT4_BG_flags (INODE_UNINIT, etc) */  
    bg_exclude_bitmap_lo : i32,   /* Exclude bitmap for snapshots */  
    bg_block_bitmap_csum_lo : i16,/* crc32c(s_uuid+grp_num+bbitmap) LE */  
    bg_inode_bitmap_csum_lo : i16,/* crc32c(s_uuid+grp_num+ibitmap) LE */  
    bg_itable_unused_lo : i16,    /* Unused inodes count */  
    bg_checksum : i16,        /* crc16(sb_uuid+group+desc) */  
    bg_block_bitmap_hi : i32, /* Blocks bitmap block MSB */  
    bg_inode_bitmap_hi : i32, /* Inodes bitmap block MSB */  
    bg_inode_table_hi : i32,  /* Inodes table block MSB */  
    bg_free_blocks_count_hi : i16,/* Free blocks count MSB */  
    bg_free_inodes_count_hi : i16,/* Free inodes count MSB */  
    bg_used_dirs_count_hi : i16,  /* Directories count MSB */  
    bg_itable_unused_hi : i16,    /* Unused inodes count MSB */  
    bg_exclude_bitmap_hi : i32,   /* Exclude bitmap block MSB */  
    bg_block_bitmap_csum_hi : i16,/* crc32c(s_uuid+grp_num+bbitmap) BE */  
    bg_inode_bitmap_csum_hi : i16,/* crc32c(s_uuid+grp_num+ibitmap) BE */  
    bg_reserved : u32  
}

#[repr(C)]
struct Ext4ExtentIdx
{  
    ei_block : i32,   /* index covers logical blocks from 'block' */  
    ei_leaf_lo : i32, /* pointer to the physical block of the next * 
    * level. leaf or next index could be there */  
    ei_leaf_hi : i16, /* high 16 bits of physical block */  
    ei_unused : i16  
}

#[repr(C)]
struct Ext4Inode
{
    i_mode : i16,
    i_uid : i16,
    i_size_lo : i32,
    i_atime : i32,
    i_ctime : i32,
    i_mtime : i32,
    i_dtime : i32,
    i_gid : i16,
    i_links_coint : i16,
    i_blocks_lo : i32,
    i_flags : i32,
    osd1 : Osd1,
    i_block : [i32; EXT4_N_BLOCKS],
    i_generation : i32,
    i_file_acl_lo : i32,
    i_size_high : i32,
    i_obso_faddr : i32,
    osd2 : Osd2,
    i_extra_isize : i16,
    i_checksum_hi : i16,
    i_ctime_extra : i32,
    i_mtime_extra : i32,
    i_atime_extra : i32,
    i_crtime : i32,
    i_crtime_extra : i32,
    i_version_hi : i32,
    i_projid : i32,
}

#[repr(C)]
struct Linux2
{
    l_i_blocks_high : i16,
    l_i_file_acl_high : i16,
    l_i_uid_high : i16,
    l_i_gid_high : i16,
    l_i_check_sum_lo : i16,
    l_i_reserved : i16
}

#[repr(C)]
struct Hurd2
{
    h_i_reserved1 : i16,
    h_i_mode_high : u16,
    h_i_uid_high : u16,
    h_i_gid_high : u16,
    h_i_author : u16
}

#[repr(C)]
struct Masix2
{
    h_i_reserved1 : i16,
    m_i_file_acl_high : i16,
    m_i_reserced2 : [u32; 2]
}

#[repr(C)]
union Osd2
{
    linux2 : core::mem::ManuallyDrop<Linux2>,
    hurd2 : core::mem::ManuallyDrop<Hurd2>,
    masix2 : core::mem::ManuallyDrop<Masix2>
}

#[repr(C)]
union Osd1 {
    l_i_version : i32,
    h_itranslator : u32,
    m_ireserved1 : u32,
}

#[repr(C)]
struct Ext4SuperBlock
{
    s_inodes_count : i32,		/* Inodes count */
    s_blocks_count_lo : i32,	/* Blocks count */
    s_r_blocks_count_lo : i32,	/* Reserved blocks count */
    s_free_blocks_count_lo : i32,	/* Free blocks count */
    s_free_inodes_count : i32,	/* Free inodes count */
    s_first_data_block : i32,	/* First Data Block */
    s_log_block_size : i32,	/* Block size */
    s_log_cluster_size : i32,	/* Allocation cluster size */
    s_blocks_per_group : i32,	/* # Blocks per group */
    s_clusters_per_group : i32,	/* # Clusters per group */
    s_inodes_per_group : i32,	/* # Inodes per group */
    s_mtime : i32,		/* Mount time */
    s_wtime : i32,		/* Write time */
    s_mnt_count : i16,		/* Mount count */
    s_max_mnt_count : i16,	/* Maximal mount count */
    s_magic : i16,		/* Magic signature */
    s_state : i16,		/* File system state */
    s_errors : i16,		/* Behaviour when detecting errors */
    s_minor_rev_level : i16,	/* minor revision level */
    s_lastcheck : i32,		/* time of last check */
    s_checkinterval : i32,	/* max. time between checks */
    s_creator_os : i32,		/* OS */
    s_rev_level : i32,		/* Revision level */
    s_def_resuid : i16,		/* Default uid for reserved blocks */
    s_def_resgid : i16,		/* Default gid for reserved blocks */
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
    s_first_ino : i32,		/* First non-reserved inode */
    s_inode_size : i16,		/* size of inode structure */
    s_block_group_nr : i16,	/* block group # of this superblock */
    s_feature_compat : i32,	/* compatible feature set */
    s_feature_incompat : i32,	/* incompatible feature set */
    s_feature_ro_compat : i32,	/* readonly-compatible feature set */
    s_uuid : [u8; 16],		/* 128-bit uuid for volume */
    s_volume_name : [c_char; EXT4_LABEL_MAX],	/* volume name */
    s_last_mounted : [c_char; 64],	/* directory where last mounted */
    s_algorithm_usage_bitmap : i32, /* For compression */
        /*
         * Performance hints.  Directory preallocation should only
         * happen if the EXT4_FEATURE_COMPAT_DIR_PREALLOC flag is on.
         */
    s_prealloc_blocks : u8,	/* Nr of blocks to try to preallocate*/
    s_prealloc_dir_blocks : u8,	/* Nr to preallocate for dirs */
    s_reserved_gdt_blocks : i16,	/* Per group desc for online growth */
        /*
         * Journaling support valid if EXT4_FEATURE_COMPAT_HAS_JOURNAL set.
         */
    s_journal_uuid : [u8; 16],	/* uuid of journal superblock */
    s_journal_inum : i32,		/* inode number of journal file */
    s_journal_dev : i32,		/* device number of journal file */
    s_last_orphan : i32,		/* start of list of inodes to delete */
    s_hash_seed : [i32; 4],		/* HTREE hash seed */
    s_def_hash_version : u8,	/* Default hash version to use */
    s_jnl_backup_type : u8,
    s_desc_size : i16,		/* size of group descriptor */
    s_default_mount_opts : i32,
    s_first_meta_bg : i32,	/* First metablock block group */
    s_mkfs_time : i32,		/* When the filesystem was created */
    s_jnl_blocks : [i32; 17],	/* Backup of the journal inode */
        /* 64bit support valid if EXT4_FEATURE_INCOMPAT_64BIT */
    s_blocks_count_hi : i32,	/* Blocks count */
    s_r_blocks_count_hi : i32,	/* Reserved blocks count */
    s_free_blocks_count_hi : i32,	/* Free blocks count */
    s_min_extra_isize : i16,	/* All inodes have at least # bytes */
    s_want_extra_isize : i16, 	/* New inodes should reserve # bytes */
    s_flags : i32,		/* Miscellaneous flags */
    s_raid_stride : i16,		/* RAID stride */
    s_mmp_update_interval : i16,  /* # seconds to wait in MMP checking */
    s_mmp_block : i64,            /* Block for multi-mount protection */
    s_raid_stripe_width : i32,    /* blocks on all data disks (N*stride)*/
    s_log_groups_per_flex : u8,  /* FLEX_BG group size */
    s_checksum_type : u8,	/* metadata checksum algorithm used */
    s_encryption_level : u8,	/* versioning level for encryption */
    s_reserved_pad : u8,		/* Padding to next 32bits */
    s_kbytes_written : i64,	/* nr of lifetime kilobytes written */
    s_snapshot_inum : i32,	/* Inode number of active snapshot */
    s_snapshot_id : i32,		/* sequential ID of active snapshot */
    s_snapshot_r_blocks_count : i64, /* reserved blocks for active
                              snapshot's future use */
    s_snapshot_list : i32,	/* inode number of the head of the
                           on-disk snapshot list */
    // #define EXT4_S_ERR_START offsetof(struct ext4_super_block, s_error_count)
    s_error_count : i32,		/* number of fs errors */
    s_first_error_time : i32,	/* first time an error happened */
    s_first_error_ino : i32,	/* inode involved in first error */
    s_first_error_block : i16,	/* block involved of first error */
    s_first_error_func : [u8; 32],	/* function where the error happened */
    s_first_error_line : i32,	/* line number where error happened */
    s_last_error_time : i32,	/* most recent time of an error */
    s_last_error_ino : i32,	/* inode involved in last error */
    s_last_error_line : i32,	/* line number where error happened */
    s_last_error_block : i64,	/* block involved of last error */
    s_last_error_func : [u8; 32],	/* function where the error happened */
    // #define EXT4_S_ERR_END offsetof(struct ext4_super_block, s_mount_opts)
    s_mount_opts : [u8; 64],
    s_usr_quota_inum : i32,	/* inode for tracking user quota */
    s_grp_quota_inum : i32,	/* inode for tracking group quota */
    s_overhead_clusters : i32,	/* overhead blocks/clusters in fs */
    s_backup_bgs : [i32; 2],	/* groups with sparse_super2 SBs */
    s_encrypt_algos : [u8; 4],	/* Encryption algorithms in use  */
    s_encrypt_pw_salt : [u8; 16],	/* Salt used for string2key algorithm */
    s_lpf_ino : i32,		/* Location of the lost+found inode */
    s_prj_quota_inum : i32,	/* inode for tracking project quota */
    s_checksum_seed : i32,	/* crc32c(uuid) if csum_seed set */
    s_wtime_hi : u8,
    s_mtime_hi : u8,
    s_mkfs_time_hi : u8,
    s_lastcheck_hi : u8,
    s_first_error_time_hi : u8,
    s_last_error_time_hi : u8,
    s_first_error_errcode : u8,
    s_last_error_errcode : u8,
    s_encoding : i16,		/* Filename charset encoding */
    s_encoding_flags : i16,	/* Filename charset encoding flags */
    s_orphan_file_inum : i32,	/* Inode for tracking orphan inodes */
    s_reserved : [i32; 94],		/* Padding to the end of the block */
    s_checksum : i32		/* crc32c(superblock) */
}

struct Ext4Extent {
	ee_block : i32,	/* exient叶子的第一个数据块号 */
	ee_len : i16,		/* exient叶子的数据块数量 */
	ee_start_hi : i16,	/* 物理数据块的高16位 */
	ee_start_lo : i32,	/* 物理数据块的低32位 */
}

