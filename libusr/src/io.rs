use crate::sys::{self, Stat};
use syscall::stat::AT_FDCWD;
use core::fmt;

const STDOUT_FILENO: i32 = 0;

pub fn stat(pathname: &str) -> Result<Stat, ()> {
    let mut buf = Stat::default();
    let res = unsafe {
        sys::sys_fstatat(AT_FDCWD, pathname, &mut buf, 0)
    };
    if res != 0 {
        todo!();
    }
    Ok(buf)
}

// print!/println! group

#[macro_export]
macro_rules! print {
    ($($args:tt)+) => ($crate::io::_print(format_args!($($args)+)))
}

#[macro_export]
macro_rules! println {
    ($($args:tt)+) => (print!("{}\n", format_args!($($args)+)))
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

pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    static mut BUFFER: [u8; 4096] = [0; 4096];
    let mut writer = BufferWriter {
        buf: unsafe { &mut BUFFER },
        pos: 0,
    };
    writer.write_fmt(args).ok();
    unsafe {
        sys::sys_write(STDOUT_FILENO, &BUFFER[..writer.pos]);
    }
}
