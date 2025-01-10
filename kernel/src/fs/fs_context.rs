use core::{alloc::Layout, ffi::{c_void, c_char}, iter::empty, ptr::{null_mut, drop_in_place}};

use alloc::{rc::Rc, string::String, sync::Arc};

use crate::kernel::{errno_base::{err_ptr, ptr_err, EINVAL, ENOMEM, ENOPARAM}, semaphore::Semaphore, Err, string::strsep};

use super::{dcache::DEntry, file::File, fs::{FileSystemType, SB_DIRSYNC, SB_LAZYTIME, SB_MANDLOCK, SB_RDONLY, SB_SYNCHRONOUS}};




static COMMON_SET_SB_FLAG : [ConstantTable; 6] = 
[
    ConstantTable { name: "dirsync", value: SB_DIRSYNC },
    ConstantTable { name: "lazytime", value: SB_LAZYTIME },
    ConstantTable { name: "mand", value: SB_MANDLOCK },
    ConstantTable { name: "ro", value: SB_RDONLY },
    ConstantTable { name: "sync", value: SB_SYNCHRONOUS },
    ConstantTable::empty()
];


static COMMON_CLEAR_SB_FLAG : [ConstantTable; 5] = 
[
    ConstantTable { name: "async", value: SB_SYNCHRONOUS },
	ConstantTable { name: "nolazytime", value: SB_LAZYTIME },
	ConstantTable { name: "nomand",	value: SB_MANDLOCK },
	ConstantTable { name: "rw",	value: SB_RDONLY },
    ConstantTable::empty()
];


fn __lookup_constant(mut tbl : *const ConstantTable, name : &str) -> *const ConstantTable
{
    unsafe
    {
        while (*tbl).name.len() > 0 {
            if (*tbl).name == name
            {
                return tbl
            }
            tbl = tbl.offset(1);
        }
        return null_mut();
    }

}

fn lookup_constant(tbl : *const ConstantTable, name : &str, not_fount : u32) -> u32
{
    unsafe
    {
        let p = __lookup_constant(tbl, name);
        if p.is_null()
        {
            not_fount
        }
        else {
            (*p).value
        }
    }
}


struct ConstantTable
{
    name : &'static str,
    value : u32
}

impl ConstantTable {
    pub const fn empty() -> Self
    {
        Self { name: "", value: 0 }
    }
}

#[derive(PartialEq, Eq)]
enum FsValueType {
	FsValueIsUndefined,
	FsValueIsFlag,		/* Value not given a value */
	FsValueIsString,		/* Value is a string */
	FsValueIsBlob,		/* Value is a binary blob */
	FsValueIsFilename,		/* Value is a filename* + dirfd */
	FsValueIsFile,		/* Value is a file* */
}

union FsParameterValue<'a>
{
    string : &'a Arc<String>,
    blob : *const c_void,
    file : *const File,
    filename : &'a String
}

pub struct FsParameter<'a, 'b>
{
    key : &'b String,
    p_type : FsValueType,
    value : FsParameterValue<'a>,
    size : usize
}

fn vfs_parse_sb_flag(fc : *mut FsContext, key : &String) -> Err
{
    unsafe
    {
        let mut token;
        token = lookup_constant(COMMON_SET_SB_FLAG.as_ptr(), &key, 0);
        if token != 0
        {
            (*fc).sb_flags |= token;
            (*fc).sb_flags_mask |= token;
            return 0;
        }

        token = lookup_constant(COMMON_CLEAR_SB_FLAG.as_ptr(), &key, 0);
        if token != 0
        {
            (*fc).sb_flags |= token;
            (*fc).sb_flags_mask |= token;
            return 0;
        }
        -ENOPARAM
    }

}

fn vfs_parse_fs_param_source(fc : *mut FsContext, param : &mut FsParameter) -> Err
{
    unsafe
    {
        if param.key != "source"
        {
            return -ENOPARAM;
        }
        if param.p_type != FsValueType::FsValueIsFlag
        {
            return -EINVAL;
        }
        if !(*fc).source.is_empty()
        {
            return -EINVAL;
        }
        (*fc).source = (*param.value.string).clone();
        0
    }
}

fn vfs_parsefs_param(fc : *mut FsContext, param : &mut FsParameter) -> Err
{
    unsafe
    {
        if param.key.is_empty()
        {
            return -EINVAL;
        }
        let mut ret = vfs_parse_sb_flag(fc, param.key);
        if ret != -ENOPARAM
        {
            return ret;
        }
        match (*(*fc).ops).parse_param {
            Some(func) => 
            {
                ret = func(fc, param);
                if ret != -ENOPARAM
                {
                    return ret;
                }
            },
            None => { },
        }
        ret = vfs_parse_fs_param_source(fc, param);
        if ret != -ENOPARAM
        {
            return ret;
        }
        -EINVAL
    }
}

pub fn vfs_parse_fs_string(fc : *mut FsContext, key : &str, value : &Arc<String>) -> Err
{
    let ksvalue = value.clone();
    let mut param = FsParameter {
        key: &String::from(key),
        value : FsParameterValue { string : &ksvalue },
        p_type : FsValueType::FsValueIsFlag,
        size : value.len()
    };
    vfs_parsefs_param(fc, &mut param)
}

pub type FsContextParseParamFn = fn(*mut FsContext, *mut FsParameter) -> Err;
pub type FsContextGetTreeFn = fn(*mut FsContext) -> Err;
pub type FsContextParseMonolithicFn = fn(*mut FsContext, *mut c_void) -> Err;

pub struct FsContextOperations
{
    pub parse_param : Option<FsContextParseParamFn>,
    pub get_tree : Option<FsContextGetTreeFn>,
    pub parse_monolithic : Option<FsContextParseMonolithicFn>
}

pub struct FsContext
{
    pub ops : *mut FsContextOperations,
    pub mutex : Semaphore,
    pub source : Arc<String>,
    pub root : *mut DEntry,
    pub fs_type : *mut FileSystemType,
    pub fs_private : *mut c_void,
    pub sb_flags : u32,
    pub sb_flags_mask : u32,
    pub purpose : FsContextPurpose,
    need_free : bool
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum FsContextPurpose {
    FsContextForMount,		/* New superblock for explicit mount */
	FsContextForSubmount,	/* New superblock for automatic submount */
	FsContextForReconfigure,	/* Superblock reconfiguration (remount) */
}

impl FsContext
{
    pub fn puts_context(context : *mut Self)
    {
        unsafe
        {
            drop_in_place(context);
            alloc::alloc::dealloc(context.cast(), Layout::new::<Self>());
        }
    }

    pub fn alloc_context(fs_type : *mut FileSystemType, refference : *mut DEntry, sb_flags : u32, sb_flags_mask : u32, purpose : FsContextPurpose) -> *mut Self
    {
        unsafe
        {
            let init_fs_context;
            let fc;
            fc = alloc::alloc::alloc(Layout::new::<FsContext>()) as *mut Self;
            if fc.is_null()
            {
                return err_ptr(-ENOMEM);
            }
            fc.write(Self { ops: null_mut(), mutex: Semaphore::new(1), source: Arc::new(String::new()), root: null_mut(), fs_type, sb_flags, sb_flags_mask, purpose, need_free: false, fs_private : null_mut() });
            match purpose {
                FsContextPurpose::FsContextForMount => 
                {

                },
                FsContextPurpose::FsContextForSubmount => 
                {

                },
                FsContextPurpose::FsContextForReconfigure => 
                {

                },
            }
            init_fs_context = (*(*fc).fs_type).init_fs_context;
            let ret =
            match init_fs_context {
                Some(func) => func(fc),
                None => { 0 },
            };
            if ret < 0
            {
                Self::puts_context(fc);
                return err_ptr(ret);
            }
            (*fc).need_free = true;
            fc
        }

    }

    pub fn context_for_mount(fs_type : *mut FileSystemType, sb_flags : u32) -> *mut Self
    {
        Self::alloc_context(fs_type, null_mut(), sb_flags, 0, FsContextPurpose::FsContextForMount)
    }
}

pub fn parse_monolithic_mount_data(fc : *mut FsContext, data : *mut c_void) -> Err
{
    unsafe
    {
        let monolithic_mount_data = match (*(*fc).ops).parse_monolithic
        {
            Some(func) => func,
            None => generic_parse_monolithic,
        };
        monolithic_mount_data(fc, data)
    }
}

fn generic_parse_monolithic(fc : *mut FsContext, data : *mut c_void) -> Err
{
    vfs_parse_monolithic_sep(fc, data, vfs_parse_comma_sep)
}

fn vfs_parse_monolithic_sep(fc : *mut FsContext, data : *mut c_void, sep : fn(*mut *mut c_char) -> *mut c_char) -> Err
{
    let options = data;
    if options.is_null()
    {
        return 0;
    } 
    todo!();
}

fn vfs_parse_comma_sep(s : *mut *mut c_char) -> *mut c_char
{
    unsafe
    {
        strsep(s, ",".as_ptr() as *mut c_char)
    }
    }
