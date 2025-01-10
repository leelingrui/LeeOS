use core::{ffi::{c_char, c_void, CStr}, ptr::null_mut, alloc::Layout};

use alloc::{alloc::{alloc, dealloc}, collections::{BTreeMap, LinkedList}, string::String, vec::Vec};

use crate::{fs::{ext4::Idx, dev, namei::sys_mknod, file::FileMode}, logk};

use super::{process::{ProcessControlBlock, Priority}, list::ListHead, io::{IdeDiskT, IdePart}};
static mut DEVICES : BTreeMap<DevT, Vec<Device>> = BTreeMap::<DevT, Vec<Device>>::new();
static mut DEVICES_DRIVER : BTreeMap<DevT, Driver> = BTreeMap::<DevT, Driver>::new();

pub type DeviceReadFn = fn(dev : *mut c_void, idx : Idx, count : usize, buf : *mut c_void, flags : u32) -> i64;
pub type DeviceWriteFn = fn(dev : *mut c_void, idx : Idx, count : usize, buf : *mut c_void,  flags : u32) -> i64;
pub type DeviceIoCtlFn = fn(dev : *mut c_void, cmd : i64, args : *mut c_void, flags : u32) -> i64;
pub type RequestPriorityFn = fn(LHS : &RequestDescriptor, RHS : &RequestDescriptor) -> bool;
pub type DevT = u32;

pub const DEV_CMD_SECTOR_START : i64 = 1;
pub const DEV_CMD_SECTOR_COUNT : i64 = 2;
pub const DEV_NAME_LEN : usize = 64;
pub const DEV_NULL : u32 = 0;

#[inline(always)]
pub const fn mkdev(major : DevT, minor : DevT) -> DevT
{
    major << 20 | minor
}

#[inline(always)]
pub fn major(dev : DevT) -> DevT
{
    dev >> 20
}

#[inline(always)]
pub fn minor(dev : DevT) -> DevT
{
    dev & 0xfffff
}

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
struct Driver
{
    pub read : Option<DeviceReadFn>,
    pub write : Option<DeviceWriteFn>,
    pub ioctl : Option<DeviceIoCtlFn>,
    pub req_priority : Option<RequestPriorityFn>,
}

#[derive(Clone, Copy)]
pub struct Device
{
    pub name : [c_char; DEV_NAME_LEN],
    pub dev : DevT,
    pub flags : u32,
    pub parent : DevT,
    pub ptr : *mut c_void,
    pub request_list : ListHead
}

pub fn get_device<'a>(dev_t : DevT) -> Option<&'a mut Device>
{
    unsafe { 
        match DEVICES.get_mut(&major(dev_t)) {
            Some(dev_list) => 
            {
                if dev_list.len() > minor(dev_t) as usize
                {
                    Some(&mut dev_list[minor(dev_t) as usize])
                }
                else {
                    None
                }
            },
            None => None,
        }

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

    fn empty_req_list(&self) -> bool
    {
        return self.request_list.next.is_null();
    }

    fn insert_request(&mut self, request : *mut RequestDescriptor) -> bool
    {
        unsafe
        {
            let mut req_list = self.request_list.next as *mut RequestDescriptor;
            match DEVICES_DRIVER.get(&major(self.dev)) {
                Some(driver) =>
                {
                    if req_list.is_null()
                    {
                        self.request_list.next = request as *mut ListHead;
                    }
                    else
                    {
                        match driver.req_priority {
                            Some(priority_fn) => 
                            {
                                loop {
                                    if priority_fn(&*req_list, &*request)
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
                            },
                            None => 
                            {
                                loop {
                                    if default_request_priority(&*req_list, &*request)
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
                            },
                        }
                    }
                    true
                }
                None => false,
            }
        }

    }
}

pub fn regist_device(dev_no : DevT, ioctl_fn : Option<DeviceIoCtlFn>, read_fn : Option<DeviceReadFn>, write_fn : Option<DeviceWriteFn>, priority_fn : Option<RequestPriorityFn>)
{
    unsafe
    {
        let driver = Driver {
            read: read_fn,
            write: write_fn,
            ioctl: ioctl_fn,
            req_priority: priority_fn,
        };
        DEVICES_DRIVER.insert(dev_no, driver);
    }
}

pub fn device_install(dev_no : DevT, ptr : *mut c_void, name : &CStr, parent : DevT, flags : u32, device_type : FileMode) -> DevT
{
    unsafe
    {
        let dev;
        match DEVICES.get_mut(&dev_no) {
            Some(target_list) => 
            {
                let minor = target_list.len() as DevT;
                let mut device = Device {
                    name: [0; DEV_NAME_LEN],
                    dev: mkdev(dev_no, minor),
                    flags,
                    parent,
                    ptr,
                    request_list: ListHead::empty(),                    
                };
                // device.request_list.init();
                compiler_builtins::mem::memcpy(device.name.as_ptr() as *mut u8, name.as_ptr() as *const u8, name.to_str().unwrap().len());
                target_list.push(device);
                dev = mkdev(dev_no, minor)

             },
            None => 
            {
                let mut target_list = Vec::<Device>::new();
                let minor = target_list.len() as DevT;
                let mut device = Device {
                    name: [0; DEV_NAME_LEN],
                    dev: mkdev(dev_no, minor),
                    flags,
                    parent,
                    ptr,
                    request_list: ListHead::empty(),                    
                };
                // device.request_list.init();
                compiler_builtins::mem::memcpy(device.name.as_ptr() as *mut u8, name.as_ptr() as *const u8, name.to_str().unwrap().len());
                target_list.push(device);
                DEVICES.insert(dev_no, target_list);
                dev = mkdev(dev_no, minor)
            },
        }
        let device_dest = String::from("/dev/") + name.to_str().unwrap();
        sys_mknod(device_dest.as_ptr().cast(), device_type, dev);
        dev
    }

}

pub fn device_ioctl(dev_t : DevT, cmd : i64, args : *mut c_void, flags : u32) -> i64
{
    unsafe
    {
        match get_device(dev_t) {
            Some(device) =>
            {
                match DEVICES_DRIVER.get(&major(dev_t)) {
                    Some(driver) => {
                        match driver.ioctl {
                            Some(ioctl) => 
                            {
                                ioctl(device.ptr, cmd, args, flags)
                            },
                            None => -1,
                        }
                    },
                    None => -1,
                }
            },
            None => -1
        }
    }
}

#[inline(always)]
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
                match get_device(request.dev_idx) {
                    Some(device) => 
                    {
                        match DEVICES_DRIVER.get(&major(request.dev_idx)) {
                            Some(driver) =>
                            {
                                match driver.read {
                                    Some(read_fn) => read_fn(device.ptr, request.idx as u64, request.count, request.buffer, request.flags),
                                    None => -1,
                                }
                                
                            },
                            None => -1,
                        }

                    },
                    None => -1,
                }
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

pub fn ide_disk_ioctl(disk : *mut IdeDiskT, cmd : i64, _args : *mut c_void, _flags : u32) -> i64
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
    match get_device(dev) {
        Some(mut device) => 
        {
            let offset = (device_ioctl(dev, DEV_CMD_SECTOR_START, null_mut(), 0) as usize) + idx as usize;
            if device.parent != 0
            {
                dev = device.parent;
                device = get_device(device.parent).unwrap();
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
        },
        None => -1,
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum DeviceType {
    Null = 0,
    Console,
    Block
}
