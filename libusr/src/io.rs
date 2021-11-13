use core::fmt;
use libsys::{
    calls::{sys_fstatat, sys_write},
    stat::{Stat, FileDescriptor},
};

// TODO populate this type
pub struct Error;

pub fn stat(pathname: &str) -> Result<Stat, Error> {
    let mut buf = Stat::default();
    // TODO error handling
    let res = unsafe { sys_fstatat(None, pathname, &mut buf, 0).unwrap() };
    Ok(buf)
}

// print!/println! group

#[macro_export]
macro_rules! print {
    ($($args:tt)+) => ($crate::io::_print($crate::sys::FileDescriptor::STDOUT, format_args!($($args)+)))
}

#[macro_export]
macro_rules! println {
    ($($args:tt)+) => (print!("{}\n", format_args!($($args)+)))
}

#[macro_export]
macro_rules! eprint {
    ($($args:tt)+) => ($crate::io::_print($crate::sys::FileDescriptor::STDERR, format_args!($($args)+)))
}

#[macro_export]
macro_rules! eprintln {
    ($($args:tt)+) => (eprint!("{}\n", format_args!($($args)+)))
}

struct BufferWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl fmt::Write for BufferWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.buf[self.pos] = byte;
            self.pos += 1;
        }
        Ok(())
    }
}

pub fn _print(fd: FileDescriptor, args: fmt::Arguments) {
    use core::fmt::Write;
    static mut BUFFER: [u8; 4096] = [0; 4096];
    let mut writer = BufferWriter {
        buf: unsafe { &mut BUFFER },
        pos: 0,
    };
    writer.write_fmt(args).ok();
    unsafe {
        sys_write(fd, &BUFFER[..writer.pos]);
    }
}
