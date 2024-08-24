use std::{
    borrow::Cow,
    ffi::{c_int, c_void, CStr},
    io::{Read, Write},
};

use ffmpeg::*;

#[repr(transparent)]
pub struct AVError(pub i32);

impl std::error::Error for AVError {}

impl std::fmt::Display for AVError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buffer = Vec::with_capacity(1024);
        f.write_str(unsafe {
            &if av_strerror(self.0, buffer.as_mut_ptr(), buffer.capacity()) < 0 {
                Cow::Borrowed("unknown error")
            } else {
                CStr::from_ptr(buffer.as_ptr()).to_string_lossy()
            }
        })
    }
}

impl std::fmt::Debug for AVError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AVError({}) = {}", self.0, self)
    }
}

pub type AVResult<T> = Result<T, AVError>;

pub trait AVI32Ext: Sized {
    fn av_void(self) -> AVResult<()>;
    fn av_i32(self) -> AVResult<i32>;
    fn av_u32(self) -> AVResult<u32> {
        self.av_i32().map(|x| x as u32)
    }
    fn av_usize(self) -> AVResult<usize> {
        self.av_i32().map(|x| x as usize)
    }
}

impl AVI32Ext for i32 {
    fn av_void(self) -> AVResult<()> {
        if self < 0 {
            Err(AVError(self))
        } else {
            Ok(())
        }
    }

    fn av_i32(self) -> AVResult<i32> {
        if self < 0 {
            Err(AVError(self))
        } else {
            Ok(self)
        }
    }
}

unsafe extern "C" fn avio_read_impl<R: Read>(user: *mut c_void, buf: *mut u8, buf_size: c_int) -> i32 {
    let reader = &mut *(user as *mut R);

    match reader.read(std::slice::from_raw_parts_mut(buf, buf_size as usize)) {
        Ok(0) => AVERROR_EOF,
        Ok(nread) => nread as i32,
        Err(error) => match error.raw_os_error() {
            Some(errno) => AVERROR(errno),
            None => AVERROR_UNKNOWN,
        },
    }
}

unsafe extern "C" fn avio_write_impl<W: Write>(user: *mut c_void, buf: *const u8, buf_size: c_int) -> i32 {
    let writer = &mut *(user as *mut W);

    match writer.write(std::slice::from_raw_parts(buf, buf_size as usize)) {
        Ok(nwritten) => nwritten as i32,
        Err(error) => match error.raw_os_error() {
            Some(errno) => AVERROR(errno),
            None => AVERROR_UNKNOWN,
        },
    }
}

pub unsafe fn read_to_avio<R: Read>(reader: Box<R>) -> *mut AVIOContext {
    let buffer = av_malloc(2048) as *mut u8;

    avio_alloc_context(
        buffer,
        2048,
        0,
        Box::leak(reader) as *mut _ as *mut c_void,
        Some(avio_read_impl::<R>),
        None,
        None,
    )
}

pub unsafe fn write_to_avio<W: Write>(writer: Box<W>) -> *mut AVIOContext {
    let buffer = av_malloc(2048) as *mut u8;


    avio_alloc_context(
        buffer,
        2048,
        1,
        Box::leak(writer) as *mut _ as *mut c_void,
        None,
        Some(avio_write_impl::<W>),
        None,
    )
}
