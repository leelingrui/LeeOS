use core::{ffi::{c_char, c_void}, alloc::Layout, ptr::null_mut, mem::size_of};

use alloc::{vec::Vec, collections::BTreeMap, alloc::dealloc};

use crate::{kernel::{console::CONSOLE, device::DevT, process::PCB, bitmap::BitMap, math::{pow, self}, buffer::Buffer}, mm::memory::PAGE_SIZE, fs::ext4::{ext4_get_logic_block_idx, ext4_inode_format}};

use super::ext4::{Idx, Ext4SuperBlock, Ext4GroupDesc, ext4_inode_read, Ext4Inode, ext4_find_entry, ext4_match_name, Ext4DirEntry2, self};
pub static mut FS : FileSystem = FileSystem::new();

pub struct FileSystem
{
    logical_part : BTreeMap<DevT, LogicalPart>,
    iroot : *mut Inode,
    imount : *mut Inode,
    root_dev : DevT
}

impl FileSystem {
    pub fn release_inode(&mut self, inode : *mut Inode)
    {
        unsafe
        {
            let logical_part = self.logical_part.get_mut(&(*inode).dev);
            match logical_part {
                Some(x) => 
                {
                    x.release_inode(inode);
                },
                None => panic!("no device {}", (*inode).dev),
            }
        }
    }

    pub fn get_iroot(&self) -> *mut Inode
    {
        self.iroot
    }

    const fn new() -> Self
    {
        Self { logical_part: BTreeMap::new(), iroot: null_mut(), root_dev: 0, imount: null_mut() }
    }

    pub fn read_inode(&mut self, inode : *mut Inode, buffer : *mut c_void, len : usize, offset : usize) -> i64
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
            self.iroot = new_sb.get_inode(2);
            self.imount = new_sb.get_inode(2);
            (*self.iroot).mount = dev;
        }

    }

    pub fn get_inode(&mut self, dev : DevT, inode_idx : Idx) -> *mut Inode
    {
        let sb = self.logical_part.get_mut(&dev);
        if sb.is_some()
        {
            sb.unwrap().get_inode(inode_idx)
        }
        else 
        {
            null_mut()
        }
    }

    fn load_group_desc(dev : DevT, idx : Idx) -> *mut Ext4GroupDesc
    {
        unsafe
        {
            let desc = unsafe { alloc::alloc::alloc(Layout::new::<Ext4GroupDesc>()) as *mut Ext4GroupDesc };
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
    pub fs_type : FSType,
    pub super_block : *mut c_void,
    pub group_desc : Vec<GroupDesc>,
    pub logic_block_size : i32,
    pub logic_block_count : usize,
    pub inode_count : usize,
    pub inode_map : BTreeMap<Idx, *mut Inode>,
    pub data_map : BTreeMap<Idx, *mut Buffer>,
    pub inode_per_group : usize,
    pub blocks_per_group : usize
}

#[derive(PartialEq)]
pub enum FSType {
    None,
    Ext4
}

pub struct Inode
{
    pub inode_block_buffer : *mut Buffer,
    pub inode_desc_ptr : *mut c_void,
    pub logical_part_ptr : *mut LogicalPart,
    pub count : u32,
    pub rx_waiter : *mut PCB,
    pub tx_waiter : *mut PCB,
    pub mount : DevT,
    pub dev : DevT,
    pub nr : Idx,
}

impl Inode {
    pub fn is_dir(&self) -> bool
    {
        unsafe
        {
            match (*self.logical_part_ptr).fs_type {
                FSType::None => panic!("unsupport fs\n"),
                FSType::Ext4 => ext4::is_dir((*(self.inode_desc_ptr as *mut Ext4Inode)).i_mode),
            }
        }
    }

    pub fn get_size(&self) -> usize
    {
        unsafe
        {
            match (*self.logical_part_ptr).fs_type {
                FSType::None => panic!("unsupport fs\n"),
                FSType::Ext4 => (*(self.inode_desc_ptr as *mut Ext4Inode)).i_size_lo as usize + (((*(self.inode_desc_ptr as *mut Ext4Inode)).i_size_lo as usize) << 32),
            }
        }
    }

    pub fn find_entry(&mut self, name : *const c_char, next : &mut *mut c_char, result_entry : &mut DirEntry)
    {
        unsafe
        {
            match (*self.logical_part_ptr).fs_type {
                FSType::None => panic!("unsupport fs\n"),
                FSType::Ext4 => ext4_find_entry(self, name, next, result_entry),
            }
        }
    }
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

    pub fn dispose(&self)
    {
        unsafe
        {
            match self.dir_entry_type {
                FSType::None => panic!("unsupport fs\n"),
                FSType::Ext4 => {
                    alloc::alloc::dealloc(self.entry_ptr as *mut u8, Layout::new::<Ext4DirEntry2>());
                },
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
            }
        }
    }

    pub fn get_entry_ptr_size(&self) -> usize{
        unsafe
        {
            match self.dir_entry_type {
                FSType::None => panic!("unsupport fs\n"),
                FSType::Ext4 => (*(self.entry_ptr as *const Ext4DirEntry2)).rec_len as usize,
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
            }
        }

    }
}

impl LogicalPart {
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

    pub fn get_buffer(&mut self, idx : Idx) -> *mut Buffer 
    {
        unsafe
        {
            let result = self.data_map.get(&idx);
            if result.is_some()
            {
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

    fn new() -> Self
    {
        Self { fs_type: FSType::None, super_block: null_mut(), group_desc: Vec::new(), logic_block_size: 0, logic_block_count: 0, inode_count: 0, dev: 0, inode_map: BTreeMap::new(), inode_per_group: 0, data_map: BTreeMap::new(), blocks_per_group: 0 }
    }

    fn read_block(&self, logic_block_no : usize) -> *mut Buffer
    {
        disk_read(self.dev,  self.logic_block_size as u64 * 2 * logic_block_no as u64, (self.logic_block_size * 2).try_into().unwrap())
    }

    fn get_buffered_inode(&self, idx : Idx) -> *mut Inode
    {
        match self.inode_map.get(&idx) {
            Some(x) => return *x,
            None => return null_mut(),
        }
    }

    pub fn release_inode(&mut self, inode : *mut Inode)
    {
        unsafe
        {
            (*inode).count -= 1;
            if (*inode).count == 0
            {
                // todo!()
            }
        }
    }

    fn add_to_buffered_inode(&mut self, inode : *mut Inode)
    {
        unsafe
        {
            self.inode_map.insert((*inode).nr, inode);
        }
    }

    fn get_free_inode(&self) -> *mut Inode
    {
        unsafe { alloc::alloc::alloc(Layout::new::<Inode>()) as *mut Inode }
    }

    #[inline]
    fn get_group_desc_no(&self, nr : Idx) -> Idx
    {
        (nr - 1) as Idx / self.inode_per_group as Idx
    }

    #[inline]
    fn inode_per_blocks(&self) -> Idx
    {
        1024 * self.inode_per_group as Idx / 256
    }
    #[inline]
    fn get_inode_logical_block(&self, mut nr : Idx) -> Idx
    {
        nr = nr / self.inode_per_blocks() + 1;
        let sb = self.super_block as *mut Ext4SuperBlock;
        unsafe {
            let desc = &self.group_desc[self.get_group_desc_no(nr) as usize];
            (desc.inode_table_offset + nr % (*sb).s_blocks_per_group as u64 / self.inode_per_blocks()).try_into().unwrap()
        }
    }

    fn get_inode(&mut self, inode_idx : Idx) -> *mut Inode
    {
        unsafe
        {
            assert!(inode_idx <= self.inode_count as Idx);
            let mut inode = self.get_buffered_inode(inode_idx);
            if !inode.is_null()
            {
                (*inode).count += 1;
                return inode;
            }
            inode = self.get_free_inode();
            (*inode).dev = self.dev;
            (*inode).nr = inode_idx;
            (*inode).count = 1;
            self.inode_map.insert(inode_idx, inode);
            let block_no = self.get_inode_logical_block(inode_idx);
            let buffer = self.read_block(block_no as usize);
            (*inode).inode_block_buffer = buffer;
            (*inode).logical_part_ptr = self;
            match self.fs_type
            {
                FSType::Ext4 =>
                {
                    ext4_inode_format(inode, buffer, self.logic_block_size, inode_idx);
                }
                _ => panic!("unsupport fs\n")
            }
            self.add_to_buffered_inode(inode);
            inode
        }
    }
}




#[inline]
fn get_device_buffer(dev : DevT, block : Idx) -> *mut Buffer
{
    unsafe
    {
        let buffer =alloc::alloc::alloc(Layout::new::<Buffer>()) as *mut Buffer;
        *buffer = Buffer::new(4096);
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

pub fn sys_write(fd : FileDescriptor, buf : *const c_void, count : usize)
{
    if fd == STDOUT
    {
        unsafe { 
            CONSOLE.write(buf as *const c_char, count);
        }
    }
}