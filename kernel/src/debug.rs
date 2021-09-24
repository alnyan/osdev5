use crate::dev::serial::SerialDevice;
use crate::sync::Spin;
use core::fmt;

struct SerialOutput<T: 'static + SerialDevice> {
    inner: &'static Spin<T>,
}

impl<T: SerialDevice> fmt::Write for SerialOutput<T> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut lock = self.inner.lock();
        for &byte in s.as_bytes() {
            unsafe {
                // TODO check for errors
                drop(lock.send(byte));
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
    use crate::arch::machine;
    use fmt::Write;

    drop(SerialOutput { inner: machine::console() }.write_fmt(args));
}
