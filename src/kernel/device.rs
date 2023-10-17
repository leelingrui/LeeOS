use core::{ffi::{c_char, c_void}, ptr::null_mut, alloc::Layout};

use alloc::alloc::{alloc, dealloc};

use crate::{fs::ext4::Idx, logk};

use super::{process::ProcessControlBlock, list::ListHead, io::{IdeDiskT, IdePart}};
const DEVICE_NR : usize = 0xff;
static mut DEVICES : [Device; DEVICE_NR] = [Device::empty(); DEVICE_NR];

pub type DeviceReadFn = fn(dev : *mut c_void, idx : Idx, count : usize, buf : *mut c_void, flags : u32) -> i64;
pub type DeviceWriteFn = fn(dev : *mut c_void, idx : Idx, count : usize, buf : *mut c_void,  flags : u32) -> i64;
pub type DeviceIoCtlFn = fn(dev : *mut c_void, cmd : i64, args : *mut c_void, flags : u32) -> i64;
pub type RequestPriorityFn = fn(LHS : &RequestDescriptor, RHS : &RequestDescriptor) -> bool;
pub type DevT = u32;

pub const DEV_CMD_SECTOR_START : i64 = 1;
pub const DEV_CMD_SECTOR_COUNT : i64 = 2;

pub enum DevReqType
{
    Read,
    Write,
}

pub struct RequestDescriptor
{
    dev_idx : DevT,
    req_type : DevReqType,
    idx : usize,
    count : usize,
    buffer : *mut c_void,
    flags : u32,
    process : *mut ProcessControlBlock,
    list_node : ListHead
}


impl RequestDescriptor {
    fn get_next(&self) -> *mut RequestDescriptor
    {
        self.list_node.next as *mut RequestDescriptor
    }

    fn get_prev(&self) -> *mut RequestDescriptor
    {
        self.list_node.prev as *mut RequestDescriptor
    }

    fn set_prev(&mut self, prev : *mut RequestDescriptor)
    {
        self.list_node.prev = prev as *mut ListHead;
    }

    fn set_next(&mut self, next : *mut RequestDescriptor)
    {
        self.list_node.next = next as *mut ListHead;
    }

}


#[derive(Clone, Copy)]
pub struct Device
{
    pub name : [c_char; 16],
    pub dev_type : DeviceType,
    pub flags : u32,
    pub parent : DevT,
    pub ptr : *mut c_void,
    pub buf : *mut c_void,
    pub read : Option<DeviceReadFn>,
    pub write : Option<DeviceWriteFn>,
    pub ioctl : Option<DeviceIoCtlFn>,
    pub req_priority : Option<RequestPriorityFn>,
    pub request_list : ListHead
}

pub fn get_device(dev_t : DevT) -> &'static mut Device
{
    unsafe { 
        assert!(DEVICES[dev_t as usize].dev_type != DeviceType::Null);
        &mut DEVICES[dev_t as usize]
    }
}

const fn default_request_priority(lhs : &RequestDescriptor, rhs : &RequestDescriptor) -> bool
{
    false
}

impl Device {
    fn get_next_request(&self) -> *mut RequestDescriptor
    {
        self.request_list.next as *mut RequestDescriptor
    }

    fn erase_request(&mut self, request : *mut RequestDescriptor)
    {
        unsafe
        {
            if (*request).get_prev().is_null()
            {
                self.request_list.next = (*request).get_next() as *mut ListHead;
                (*request).set_prev(null_mut());
            }
            else {
                (*(*request).get_prev()).set_next((*request).get_next());
                if !(*request).get_next().is_null()
                {
                    (*(*request).get_next()).set_prev((*request).get_prev());
                }
            }
        }
    }

    const fn empty() -> Self
    {
        Self { name: ['n' as i8, 'u' as i8, 'l' as i8, 'l' as i8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], dev_type: DeviceType::Null, flags: 0, buf: null_mut(), read:Option::None, write: Option::None, ioctl: Option::None, parent: 0, ptr: null_mut(), request_list: ListHead::empty(), req_priority: Some(default_request_priority as RequestPriorityFn) }
    }

    fn empty_req_list(&self) -> bool
    {
        return self.request_list.next.is_null();
    }

    fn insert_request(&mut self, request : *mut RequestDescriptor)
    {
        unsafe
        {
            let mut req_list = self.request_list.next as *mut RequestDescriptor;
            if req_list.is_null()
            {
                self.request_list.next = request as *mut ListHead;
            }
            else
            {
                loop {
                    if (self.req_priority.unwrap())(&*req_list, &*request)
                    {
                        (*(*req_list).get_prev()).set_next(request);
                        (*req_list).set_prev((*req_list).get_prev());
                        (*req_list).set_next(req_list);
                        (*req_list).set_prev(request);
                        break;
                    }
                    let next = (*req_list).get_next();
                    if next.is_null()
                    {
                        (*req_list).set_next(request);
                        (*request).set_prev(req_list);
                        break;
                    }
                    else {
                        req_list = next;
                    }
                }
            }
        }

    }
}

fn get_null_device_no() -> usize
{
    unsafe
    {
        let mut var = 1;
        while var < DEVICE_NR {
            if DEVICES[var].dev_type == DeviceType::Null
            {
                return var;
            }
            var += 1;
        }
        return 0;
    }

}

pub fn device_install(mut dev_no : DevT, dev_type : DeviceType, ptr : *mut c_void, name : &str, parent : DevT, flags : u32, ioctl_fn : Option<DeviceIoCtlFn>, read_fn : Option<DeviceReadFn>, write_fn : Option<DeviceWriteFn>) -> DevT
{
    unsafe
    {
        if dev_no == 0
        {
            dev_no = get_null_device_no() as DevT;
            if dev_no != 0
            {
                DEVICES[dev_no as usize].dev_type = dev_type;
                DEVICES[dev_no as usize].flags = flags;
                compiler_builtins::mem::memcpy(DEVICES[dev_no as usize].name.as_mut_ptr() as *mut u8, ptr as *mut u8, 16);
                DEVICES[dev_no as usize].ioctl = ioctl_fn;
                DEVICES[dev_no as usize].read = read_fn;
                DEVICES[dev_no as usize].write = write_fn;
                DEVICES[dev_no as usize].parent = parent;
                DEVICES[dev_no as usize].ptr = ptr;
                return dev_no;
            }
        }
        else {
            if DEVICES[dev_no as usize].dev_type != DeviceType::Null
            {
                DEVICES[dev_no as usize].dev_type = dev_type;
                DEVICES[dev_no as usize].flags = flags;
                compiler_builtins::mem::memcpy(DEVICES[dev_no as usize].name.as_mut_ptr() as *mut u8, ptr as *mut u8, 16);
                DEVICES[dev_no as usize].ioctl = ioctl_fn;
                DEVICES[dev_no as usize].read = read_fn;
                DEVICES[dev_no as usize].write = write_fn;
                DEVICES[dev_no as usize].parent = parent;
                DEVICES[dev_no as usize].ptr = ptr;
                return dev_no;
            }
        }
        return 0;
    }

}

pub fn device_ioctl(dev_t : DevT, cmd : i64, args : *mut c_void, flags : u32) -> i64
{
    let device = get_device(dev_t);
    if device.ioctl.is_none()
    {
        return -1;
    }
    else {
        unsafe {
            (device.ioctl.unwrap())(device.ptr, cmd, args, flags)            
        }
    }
}

#[inline]
fn create_request(buffer : *mut c_void, count : usize, dev : u32, offset : usize) -> *mut RequestDescriptor
{
    unsafe
    {
        let request =  alloc(Layout::new::<RequestDescriptor>()) as *mut RequestDescriptor;
        (*request).buffer = buffer;
        (*request).count = count;
        (*request).dev_idx = dev;
        (*request).process = null_mut();
        (*request).idx = offset as usize;
        request
    }
}

fn do_request(request : &mut RequestDescriptor) -> i64
{
    unsafe
    {
        match request.req_type {
            DevReqType::Read => { 
                let device = get_device(request.dev_idx);
                (device.read.unwrap())(device.ptr, request.idx as u64, request.count, request.buffer, request.flags)
            },
            DevReqType::Write => todo!(),
        }
    }
}

pub fn ide_part_ioctl(part : *mut IdePart, cmd : i64, _args : *mut c_void,_flagss : u32) -> i64
{
    unsafe
    {
        match cmd {
            DEV_CMD_SECTOR_START => { (*part).start as i64 },
            DEV_CMD_SECTOR_COUNT => { (*part).count as i64 },
            _ => { panic!("unknow device command: {}", cmd) }
        }
    }

}

pub fn ide_disk_ioctl(disk : *mut IdeDiskT, cmd : i64, _args : *mut c_void,_flagss : u32) -> i64
{
    unsafe
    {
        match cmd {
            DEV_CMD_SECTOR_START => { 0 },
            DEV_CMD_SECTOR_COUNT => { (*disk).total_lba as i64 },
            _ => { panic!("unknow device command: {}", cmd) }
        }
    }

}

pub fn device_request(mut dev : DevT, buffer : *mut c_void, count : usize, idx : Idx, _flags : u32,_req_typee : DevReqType) -> i64
{
    let mut device = get_device(dev);
    assert!(device.dev_type == DeviceType::Block);
    let offset = (device_ioctl(dev, DEV_CMD_SECTOR_START, null_mut(), 0) as usize) + idx as usize;
    if device.parent != 0
    {
        dev = device.parent;
        device = get_device(device.parent);
    }
    let request = create_request(buffer, count, dev, offset);
    logk!("dev {}, request idx {}\n", dev, offset);
    let empty = device.empty_req_list();
    device.insert_request(request);
    if !empty
    {
        todo!()
    }
    let result;
    unsafe {
        result = do_request(&mut *request);
    }
    device.erase_request(request);
    let next_request = device.get_next_request();
    if !next_request.is_null()
    {
        todo!()
    }
    unsafe
    {
        dealloc(request as *mut u8, Layout::new::<RequestDescriptor>());
    }
    result
}

#[derive(Clone, Copy, PartialEq)]
pub enum DeviceType {
    Null = 0,
    Console,
    Block
}