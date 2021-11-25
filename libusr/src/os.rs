use libsys::debug::TraceLevel;
use crate::sys;
use core::fmt;

#[macro_export]
macro_rules! trace {
    ($level:expr, $($args:tt)+) => ($crate::os::_trace($level, format_args!($($args)+)))
}


#[macro_export]
macro_rules! trace_debug {
    ($($args:tt)+) => ($crate::os::_trace($crate::sys::debug::TraceLevel::Debug, format_args!($($args)+)))
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

pub fn _trace(level: TraceLevel, args: fmt::Arguments) {
    use core::fmt::Write;
    static mut BUFFER: [u8; 4096] = [0; 4096];
    let mut writer = BufferWriter {
        buf: unsafe { &mut BUFFER },
        pos: 0,
    };
    writer.write_fmt(args).ok();
    sys::sys_ex_debug_trace(level, unsafe { &BUFFER[..writer.pos] }).ok();
}
