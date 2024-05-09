use core::{alloc::Layout, ffi::{c_char, c_void, CStr}, intrinsics::unlikely, mem::size_of, ptr::null_mut, sync::atomic::AtomicI64};

use alloc::{alloc::dealloc, collections::BTreeMap, string::String, vec::Vec};
use bitflags::Flags;

use crate::{fs::ext4::{ext4_get_logic_block_idx, ext4_inode_desc_get}, kernel::{bitmap::BitMap, buffer::Buffer, console::CONSOLE, device::DevT, errno_base::{EEXIST, EFAULT, ENOMEM, EPERM}, list::ListHead, math::{self, pow}, process::{Gid, Uid, PCB}, sched::get_current_running_process, semaphore::RWLock, Err, Off}, mm::memory::PAGE_SIZE, printk};

use super::{dcache::{DEntry, DEntryOperations}, ext4::{self, ext4_find_entry, ext4_inode_block_read, ext4_inode_read, ext4_match_name, Ext4DirEntry2, Ext4GroupDesc, Ext4Inode, Ext4SuperBlock, Idx}, fs::AddressSpace, inode::Inode, mnt_idmapping::MntIdmap, namei::{named, namei}};
pub static mut FS : FileSystem = FileSystem::new();


bitflags::bitflags! {
    pub struct FileMode : u16
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

pub struct FileSystem
{
    logical_part : BTreeMap<DevT, LogicalPart>,
    iroot : *mut DEntry,
    imount : *mut DEntry,
    root_dev : DevT
}

bitflags::bitflags!
{
    pub struct FSPermission : u16
    {
        const IRWXU = 0o700;// 宿主可以读、写、执行/搜索
        const IRUSR = 0o400;// 宿主读许可
        const IWUSR = 0o200;// 宿主写许可
        const IXUSR = 0o100;// 宿主执行/搜索许可
        const IRWXG = 0o070; // 组成员可以读、写、执行/搜索
        const IRGRP = 0o040; // 组成员读许可
        const IWGRP = 0o020; // 组成员写许可
        const IXGRP = 0o010; // 组成员执行/搜索许可
        const IRWXO = 0o007; // 其他人读、写、执行/搜索许可
        const IROTH = 0o004; // 其他人读许可
        const IWOTH = 0o002; // 其他人写许可
        const IXOTH = 0o001; // 其他人执行/搜索许可
        const MASK = 0o777;
        const EXEC = Self::IXOTH.bits();
        const READ = Self::IROTH.bits();
        const WRITE = Self::IWOTH.bits();
    }
}

bitflags::bitflags!
{
    pub struct FileFlag : u64
    {
        const O_RDONLY = 00;      // 只读方式
        const O_WRONLY = 01;      // 只写方式
        const O_RDWR = 02;        // 读写方式
        const O_ACCMODE = 03;     // 文件访问模式屏蔽码
        const O_CREAT = 00100;    // 如果文件不存在就创建
        const O_EXCL = 00200;     // 独占使用文件标志
        const O_NOCTTY = 00400;   // 不分配控制终端
        const O_TRUNC = 01000;    // 若文件已存在且是写操作，则长度截为 0
        const O_APPEND = 02000;   // 以添加方式打开，文件指针置为文件尾
        const O_NONBLOCK = 04000; // 非阻塞方式打开和操作文件
    }
}
pub struct File
{
    pub flag : FileFlag,
    pub offset : usize,
    pub inode : *mut Inode,
    pub f_mapping : *mut AddressSpace
}

impl File {
    pub fn new() -> Self
    {
        Self { inode: null_mut(), flag: FileFlag::empty(), offset: 0, f_mapping: null_mut() }
    }

    pub fn get_inode(&self) -> *mut Inode
    {
        self.inode
    }
}

impl FileSystem {

    pub fn read_inode_logic_block(&mut self, inode_t : *mut Inode, block_idx : Idx) -> *mut Buffer
    {
        unsafe
        {
            let logic_part = self.logical_part.get_mut(&(*inode_t).dev);
            match logic_part {
                Some(part) => 
                {
                    match part.fs_type {
                        FSType::Ext4 => ext4_inode_block_read(part, inode_t, block_idx),
                        _ => panic!("unsupport fs type!\n"),
                    }
                },
                None => null_mut(),
            }
        }
    }

    pub fn read_file_logic_block(&mut self, file_t : *mut File, block_idx : Idx) -> *mut Buffer
    {
        unsafe
        {
            let logic_part = self.logical_part.get_mut(&(*(*file_t).inode).dev);
            match logic_part {
                Some(part) => 
                {
                    match part.fs_type {
                        FSType::Ext4 => ext4_inode_block_read(part, (*file_t).inode, block_idx),
                        _ => panic!("unsupport fs type!\n"),
                    }
                },
                None => null_mut(),
            }
        }
    }

    pub fn read_file(&mut self, file_t : *mut File, buffer : *mut c_void, len : usize, offset : Off) -> i64
    {
        unsafe
        {
            // todo!() check file readable
            self.read_inode((*file_t).inode, buffer, len, offset)
            
        }
    }

    pub fn release_file(&mut self, file_t : *mut File)
    {
        unsafe
        {
            if file_t.is_null()
            {
                return;
            }
            let logical_part = self.logical_part.get_mut(&(*(*file_t).inode).dev);
            match logical_part {
                Some(x) => 
                {
                    x.release_file(file_t);
                },
                None => panic!("no device {}", (*(*file_t).inode).dev),
            }
        }
    }

    pub fn mknod(&mut self, name : *mut c_char, mode : FileMode) -> Err
    {
        unsafe
        {
            let mut next = null_mut();
            let parent = named(name, &mut next);
            if parent.is_null()
            {
                return -EEXIST;
            }
            let idmap = MntIdmap::new();
            let mut child = (*parent).new_child(&String::from(CStr::from_ptr(next).to_str().unwrap()));
            if unlikely(child.is_null())
            {
                return -ENOMEM;
            }
            let old = match (*(*(*parent).d_inode).i_operations).lookup
            {
                Some(lookup) => lookup((*parent).d_inode, child, 0),
                None => return -EFAULT,
            };
            
            if unlikely(old.is_null())
            {
                (*child).dput();
                child = old;
            }

            Self::do_mknodat(idmap, (*parent).d_inode, child, mode, (*(*parent).d_inode).dev)
        }
    }

    pub fn do_mknodat(idmap : *mut MntIdmap, dir : *mut Inode, dentry : *mut DEntry, mode : FileMode, dev : DevT) -> Err
    {
        unsafe
        {
            if (*dir).i_mode.intersects(FileMode::IFDIR)
            {
                return -EPERM;
            }
            if unlikely((*(*dir).i_operations).mknod.is_none()) {
                return -EPERM;
            }
            (*(*dir).i_operations).mknod.unwrap()(idmap, dir, dentry, mode, dev)
            // (*(*dir).operations)
            // (*dentry).new_child(name, inode)
        }

    }

    pub fn open_file(&mut self, file_name : *const c_char, flags : FileFlag) -> *mut File
    {
        unsafe
        {
            let dentry = namei(file_name);
            let file_t = alloc::alloc::alloc(Layout::new::<File>()) as *mut File;
            (*file_t).inode = (*dentry).d_inode;
            (*file_t).flag = flags;

            file_t
        }

    }

    pub fn get_froot(&self) -> *mut DEntry
    {
        unsafe {
            self.iroot
        }

    }

    const fn new() -> Self
    {
        Self { logical_part: BTreeMap::new(), iroot: null_mut(), root_dev: 0, imount: null_mut() }
    }

    pub fn read_inode(&mut self, inode : *mut Inode, buffer : *mut c_void, len : usize, offset : Off) -> i64
    {
        unsafe
        {
            let logic_part = self.logical_part.get_mut(&(*inode).dev);
            if logic_part.is_some()
            {
                logic_part.unwrap().read_inode(inode, buffer, len, offset)
            }
            else {
                panic!("not fund device {}\n", &(*inode).dev);
            }
        }

    }

    pub fn load_root_super_block(&mut self, dev : DevT, super_block : *mut c_void)
    {
        unsafe
        {
            let sb = super_block as *mut Ext4SuperBlock;
            self.logical_part.insert(dev ,LogicalPart::new());
            self.root_dev = dev;
            let new_sb = self.logical_part.get_mut(&dev).unwrap();
            new_sb.super_block = sb as *mut c_void;
            new_sb.fs_type = FSType::Ext4;
            new_sb.dev = dev;
            new_sb.logic_block_size = pow(2.0, (*sb).s_log_block_size.into()) as i32;
            new_sb.logic_block_count = (((*sb).s_blocks_count_hi as usize) << 32) + (*sb).s_blocks_count_lo as usize;
            new_sb.inode_count = (*sb).s_inodes_count as usize;
            new_sb.inode_per_group = (*sb).s_inodes_per_group as usize;
            new_sb.blocks_per_group = (*sb).s_blocks_per_group as usize;
            let mut var = 0;
            let group_num = ((*sb).s_blocks_count_lo as i64 + (((*sb).s_blocks_count_hi as i64) << 32)) / (*sb).s_blocks_per_group as i64;
            // init group desc
            while var <= group_num {
                let desc = Self::load_group_desc(dev, (8 + 2 * var * (*sb).s_blocks_per_group as i64).try_into().unwrap());
                new_sb.group_desc.push(GroupDesc::new(new_sb));
                let new_desc = new_sb.group_desc.last_mut().unwrap();
                new_desc.group_desc_ptr = desc;
                new_desc.load_bitmaps();
                new_desc.inode_table_offset = (((*desc).bg_inode_table_hi as u64) << 32) + (*desc).bg_inode_table_lo as u64;
                new_desc.data_block_start = new_desc.inode_table_offset as usize + math::upround((*sb).s_inodes_per_group as u64 * 256, new_sb.logic_block_size as u64 * 1024) as usize / (new_sb.logic_block_size as usize * 1024);
                var += 1;
            }
            // get root dir
            self.iroot = self.get_file_by_inode_id(dev, 2, FileFlag::empty());
            // self.imount = FS.get_file(dev, 2, FileFlag::empty());
            // (*(*self.iroot).inode).mount = dev;
        }

    }

    pub fn get_file_by_inode_id(&mut self, dev : DevT, inode_idx : Idx, file_flag : FileFlag) -> *mut DEntry
    {
        unsafe
        {
            match self.logical_part.get_mut(&dev) {
                Some(sb) => 
                {
                    let dentry = DEntry::empty(null_mut());
                    let inode = sb.get_inode(inode_idx);
                    (*dentry).d_inode = inode;
                    dentry
                },
                None => null_mut(),
            } 
        }
    }

    fn load_group_desc(dev : DevT, idx : Idx) -> *mut Ext4GroupDesc
    {
        unsafe
        {
            let desc = alloc::alloc::alloc(Layout::new::<Ext4GroupDesc>()) as *mut Ext4GroupDesc;
            let src = disk_read(dev, idx, 2);
            (*src).read_from_buffer(desc as *mut c_void, 0, size_of::<Ext4GroupDesc>());
            desc
        }

    }
}

pub type FileDescriptor = u32;
pub const STDIN : u32 = 0;
pub const STDOUT : u32 = 1;
pub const STDERR : u32 = 2;
pub const EOF : i64 = -1;


pub struct GroupDesc
{
    logical_block_bitmap : BitMap,
    inode_bitmap : BitMap,
    group_desc_ptr : *mut Ext4GroupDesc,
    group_desc_no : u32,
    parent : *const LogicalPart,
    inode_table_offset : Idx,
    pub data_block_start : usize
}

impl GroupDesc {
    fn new(parent : &LogicalPart) -> Self
    {
        Self { logical_block_bitmap: BitMap::null_bitmap(), inode_bitmap: BitMap::null_bitmap(), group_desc_ptr: null_mut(), group_desc_no: 0, parent: parent as *const LogicalPart, inode_table_offset: 0, data_block_start: 0 }
    }

    fn load_bitmaps(&mut self)
    {
        unsafe
        {
            assert!(!self.group_desc_ptr.is_null());
            let block_map_buffer = (*self.parent).read_block(((*self.group_desc_ptr).bg_block_bitmap_hi as usize) << 32 + (*self.group_desc_ptr).bg_block_bitmap_lo as usize);
            let block_map = alloc::alloc::alloc(Layout::new::<[c_void; PAGE_SIZE]>());
            (*block_map_buffer).read_from_buffer(block_map as *mut c_void, 0, 1024 * (*self.parent).logic_block_size as usize);
            self.logical_block_bitmap.reset_bitmap(block_map, (*((*self.parent).super_block as *const Ext4SuperBlock)).s_blocks_per_group as usize);
            let inode_map_buffer = (*self.parent).read_block(((*self.group_desc_ptr).bg_inode_bitmap_hi as usize) << 32 + (*self.group_desc_ptr).bg_inode_bitmap_lo as usize);
            let inode_map = alloc::alloc::alloc(Layout::new::<[c_void; PAGE_SIZE]>());
            (*inode_map_buffer).read_from_buffer(block_map as *mut c_void, 0, 1024 * (*self.parent).logic_block_size as usize);
            self.inode_bitmap.reset_bitmap(inode_map, (*((*self.parent).super_block as *const Ext4SuperBlock)).s_inodes_per_group as usize)
        }
    }
}



pub struct LogicalPart
{
    pub dev : DevT,
    pub s_d_op : *mut DEntryOperations,
    pub fs_type : FSType,
    pub super_block : *mut c_void,
    pub group_desc : Vec<GroupDesc>,
    pub logic_block_size : i32,
    pub logic_block_count : usize,
    pub inode_count : usize,
    pub data_map : BTreeMap<Idx, *mut Buffer>,
    pub inode_per_group : usize,
    pub blocks_per_group : usize
}

#[derive(PartialEq)]
pub enum FSType {
    None,
    Ext4,
    Shmem
}

pub struct DirEntry
{
    pub dir_entry_type : FSType,
    pub entry_ptr : *mut c_void
}

impl DirEntry {
    pub fn empty() -> Self
    {
        Self { dir_entry_type: FSType::None, entry_ptr: null_mut() }
    }

    pub fn print_entry_name(&self)
    {
        unsafe
        {
            match self.dir_entry_type {
                FSType::None => panic!("unsupport fs\n"),
                FSType::Ext4 => {
                    
                    printk!("entery file name :{}\n", String::from_raw_parts((*(self.entry_ptr as *mut Ext4DirEntry2)).name.as_mut_ptr() as *mut u8, self.name_length(), self.name_length()));
                },
                FSType::Shmem => unimplemented!()
            }
        }
    }

    pub fn to_next_entry(&mut self)
    {
        unsafe
        {
            match self.dir_entry_type {
                FSType::None => panic!("unsupport fs\n"),
                FSType::Ext4 => {
                    self.entry_ptr = self.entry_ptr.offset((*(self.entry_ptr as *mut Ext4DirEntry2)).rec_len as isize)
                },
                FSType::Shmem => unimplemented!()
            }
        }
    }

    pub fn name_length(&self) -> usize
    {
        unsafe
        {
            match self.dir_entry_type {
                FSType::None => panic!("unsupport fs\n"),
                FSType::Ext4 => {
                    (*(self.entry_ptr as *mut Ext4DirEntry2)).name_len as usize
                },
                FSType::Shmem => unimplemented!()
            }
        }
    }

    pub fn dispose(&self)
    {
        unsafe
        {
            match self.dir_entry_type {
                FSType::None => panic!("unsupport fs\n"),
                FSType::Ext4 => {
                    alloc::alloc::dealloc(self.entry_ptr as *mut u8, Layout::new::<Ext4DirEntry2>());
                },
                FSType::Shmem => unimplemented!()
            }
        }
    }

    pub fn new(entry_ptr : *mut c_void, dir_entry_type : FSType) -> Self
    {
        Self { dir_entry_type, entry_ptr }
    }

    pub fn get_entry_ptr(&self) -> *mut c_void
    {
        self.entry_ptr
    }

    pub fn get_entry_point_to(&self) -> Idx
    {
        unsafe
        {
            match self.dir_entry_type {
                FSType::None => panic!("unsupport fs\n"),
                FSType::Ext4 => (*(self.entry_ptr as *const Ext4DirEntry2)).inode as Idx,
                FSType::Shmem => unimplemented!()
            }
        }
    }

    pub fn get_entry_ptr_size(&self) -> usize{
        unsafe
        {
            match self.dir_entry_type {
                FSType::None => panic!("unsupport fs\n"),
                FSType::Ext4 => (*(self.entry_ptr as *const Ext4DirEntry2)).rec_len as usize,
                FSType::Shmem => unimplemented!()
            }
        }
    }

    pub fn match_name(&self, name : *const c_char, next : &mut *mut c_char) -> bool
    {
        unsafe
        {
            match self.dir_entry_type {
                FSType::None => panic!("unsupport fs\n"),
                FSType::Ext4 => ext4_match_name(name, (*(self.entry_ptr as *const Ext4DirEntry2)).name.as_ptr(), next),
                FSType::Shmem => unimplemented!()
            }
        }

    }
}

impl LogicalPart {
    pub fn release_file(&mut self, file_t : *mut File)
    {
        unsafe
        {
            self.release_inode((*file_t).inode);
            dealloc(file_t as *mut u8, Layout::new::<File>());
        }
    }

    pub fn release_buffer(&mut self, buffer : *mut Buffer)
    {
        unsafe
        {
            if buffer.is_null()
            {
                panic!("try release null buffer\n");
            }
            (*buffer).count -= 1;
            if (*buffer).count != 0
            {
                return;
            }
            if (*buffer).dirty
            {
                (*buffer).write_to_device();
            }
            self.data_map.remove(&(*buffer).get_idx());
            (*buffer).dispose();
            
        }

    }

    pub fn open_file(&mut self, nr : Idx, flag : FileFlag) -> *mut File
    {
        unsafe
        {

            let inode = self.get_inode(nr);
            let f_struct = alloc::alloc::alloc(Layout::new::<File>()) as *mut File;
            (*f_struct) = File::new();
            if !f_struct.is_null()
            {
                (*f_struct).inode = inode;
                (*f_struct).flag = flag;
            }
            f_struct
        }
    }

    pub fn get_buffer(&mut self, idx : Idx) -> *mut Buffer 
    {
        unsafe
        {
            let result = self.data_map.get(&idx);
            if result.is_some()
            {
                (**result.unwrap()).count += 1;
                *result.unwrap()
            }
            else {
                let buff = alloc::alloc::alloc(Layout::new::<Buffer>()) as *mut Buffer;
                *buff = Buffer::new(self.logic_block_size as usize * 1024);
                self.data_map.insert(idx, buff);
                buff
            }
        }

    }

    pub fn get_logic_block_idx(&mut self, inode : *mut Inode, idx : Idx, create : bool) -> Idx
    {
        assert!(self.logic_block_count as u64 >= idx);
        match self.fs_type {
            FSType::Ext4 => ext4_get_logic_block_idx(self, inode, idx, create),
            _ => panic!("unsupport fs type\n")
        }
    }

    pub fn read_inode(&mut self, inode : *mut Inode, buffer : *mut c_void, len : usize, offset : usize) -> i64
    {
        match self.fs_type {
            FSType::Ext4 => ext4_inode_read(self, inode, buffer, len, offset),
            _ => panic!("unsupport fs type!\n"),
        }
    }

    pub fn new() -> Self
    {
        Self { fs_type: FSType::None, super_block: null_mut(), group_desc: Vec::new(), logic_block_size: 0, logic_block_count: 0, inode_count: 0, dev: 0, inode_per_group: 0, data_map: BTreeMap::new(), blocks_per_group: 0, s_d_op: null_mut() }
    }

    pub fn read_block(&self, logic_block_no : usize) -> *mut Buffer
    {
        disk_read(self.dev,  self.logic_block_size as u64 * 2 * logic_block_no as u64, (self.logic_block_size * 2).try_into().unwrap())
    }

    pub fn release_inode(&mut self, inode : *mut Inode)
    {
        unsafe
        {
            (*inode).count -= 1;
            if (*inode).count == 0
            {
                alloc::alloc::dealloc(inode as *mut u8, Layout::new::<Inode>());
            }
        }
    }

    fn get_free_inode(&self) -> *mut Inode
    {
        unsafe { alloc::alloc::alloc(Layout::new::<Inode>()) as *mut Inode }
    }

    #[inline(always)]
    fn get_group_desc_no(&self, nr : Idx) -> Idx
    {
        (nr - 1) as Idx / self.inode_per_group as Idx
    }

    #[inline(always)]
    fn inode_per_blocks(&self) -> Idx
    {
        1024 * self.inode_per_group as Idx / 256
    }

    #[inline(always)]
    pub fn get_inode_logical_block(&self, mut nr : Idx) -> Idx
    {
        nr = nr / self.inode_per_blocks() + 1;
        let sb = self.super_block as *mut Ext4SuperBlock;
        unsafe {
            let desc = &self.group_desc[self.get_group_desc_no(nr) as usize];
            (desc.inode_table_offset + nr % (*sb).s_blocks_per_group as u64 / self.inode_per_blocks()).try_into().unwrap()
        }
    }

    pub fn get_inode(&mut self, inode_idx : Idx) -> *mut Inode
    {
        unsafe
        {
            assert!(inode_idx <= self.inode_count as Idx);
            let inode = self.get_free_inode();
            (*inode).dev = self.dev;
            (*inode).nr = inode_idx;
            (*inode).count += 1;
            (*inode).logical_part_ptr = self;
            match self.fs_type
            {
                FSType::Ext4 =>
                {
                    ext4_inode_desc_get(self, inode, self.logic_block_size, inode_idx)
                }
                _ => panic!("unsupport fs\n")
            }
            // self.buffer_opened_file(inode);
            inode
        }
    }
}

pub fn sys_open(file_name : *const c_char, flags : FileFlag, mode : FSPermission)
{
    let pcb = get_current_running_process();
    
}


#[inline(always)]
fn get_device_buffer(dev : DevT, block : Idx) -> *mut Buffer
{
    unsafe
    {
        let logic_part = FS.logical_part.get_mut(&dev);
        let buffer;
        if logic_part.is_some()
        {
            buffer = logic_part.unwrap().get_buffer(block);
        }
        else
        {
            return null_mut();
        }
        buffer
    }
}

pub fn disk_read(dev : DevT, idx : Idx, blocks : usize) -> *mut Buffer
{
    unsafe
    {
        let buffer = get_device_buffer(dev, idx);
        (*buffer).read_from_device(dev, idx, blocks);
        buffer
    }
}

pub fn early_disk_read(dev : DevT, idx : Idx, blocks : usize) -> *mut Buffer
{
    unsafe
    {
        let buffer = alloc::alloc::alloc(Layout::new::<Buffer>()) as *mut Buffer;
        (*buffer) = Buffer::new(4096);
        (*buffer).read_from_device(dev, idx, blocks);
        buffer
    }
}

pub fn sys_write(fd : FileDescriptor, buf : *const c_void, count : usize)
{
    if fd == STDOUT
    {
        unsafe { 
            CONSOLE.write(buf as *const c_char, count);
        }
    }
}