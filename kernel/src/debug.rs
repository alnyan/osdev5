use core::fmt;

struct Output;

impl fmt::Write for Output {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &byte in s.as_bytes() {
            // XXX
            unsafe {
                core::ptr::write_volatile(0x09000000 as *mut u32, byte as u32);
            }
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! debug {
    ($($it:tt)+) => ($crate::debug::_debug(format_args!($($it)+)))
}

#[macro_export]
macro_rules! debugln {
    ($($it:tt)+) => (debug!("{}\n", format_args!($($it)+)))
}

pub fn _debug(args: fmt::Arguments) {
    use fmt::Write;
    drop(Output {}.write_fmt(args));
}
