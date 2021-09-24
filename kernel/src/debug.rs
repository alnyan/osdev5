//! Debug output module.
//!
//! The module provides [debug!] and [debugln!] macros
//! which can be used in similar way to print! and
//! println! from std.
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
            // TODO check for errors
            lock.send(byte).ok();
        }
        Ok(())
    }
}

/// Writes a debug message to debug output
#[macro_export]
macro_rules! debug {
    ($($it:tt)+) => ($crate::debug::_debug(format_args!($($it)+)))
}

/// Writes a debug message, followed by a newline, to debug output
///
/// See [debug!]
#[macro_export]
macro_rules! debugln {
    ($($it:tt)+) => (debug!("{}\n", format_args!($($it)+)))
}

#[doc(hidden)]
pub fn _debug(args: fmt::Arguments) {
    use crate::arch::machine;
    use fmt::Write;

    SerialOutput { inner: machine::console() }.write_fmt(args).ok();
}
