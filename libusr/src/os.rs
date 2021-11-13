use crate::sys;
use core::fmt;
use core::mem::{size_of, MaybeUninit};
use libsys::{ioctl::IoctlCmd, stat::FileDescriptor, termios::Termios};

pub fn get_tty_attrs(fd: FileDescriptor) -> Result<Termios, &'static str> {
    let mut termios = MaybeUninit::<Termios>::uninit();
    let res = unsafe {
        sys::sys_ioctl(
            fd,
            IoctlCmd::TtyGetAttributes,
            termios.as_mut_ptr() as usize,
            size_of::<Termios>(),
        )
    };
    if res != size_of::<Termios>() as isize {
        return Err("Failed");
    }
    Ok(unsafe { termios.assume_init() })
}

pub fn set_tty_attrs(fd: FileDescriptor, attrs: &Termios) -> Result<(), &'static str> {
    let res = unsafe {
        sys::sys_ioctl(
            fd,
            IoctlCmd::TtySetAttributes,
            attrs as *const _ as usize,
            size_of::<Termios>(),
        )
    };
    if res != size_of::<Termios>() as isize {
        return Err("Failed");
    }
    Ok(())
}

#[macro_export]
macro_rules! trace {
    ($($args:tt)+) => ($crate::os::_trace(format_args!($($args)+)))
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

pub fn _trace(args: fmt::Arguments) {
    use core::fmt::Write;
    static mut BUFFER: [u8; 4096] = [0; 4096];
    let mut writer = BufferWriter {
        buf: unsafe { &mut BUFFER },
        pos: 0,
    };
    writer.write_fmt(args).ok();
    unsafe {
        sys::sys_ex_debug_trace(&BUFFER[..writer.pos]);
    }
}
