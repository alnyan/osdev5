//! Debug output module.
//!
//! The module provides [debug!] and [debugln!] macros
//! which can be used in similar way to print! and
//! println! from std.

use crate::dev::serial::SerialDevice;
use core::fmt;

struct SerialOutput<T: 'static + SerialDevice> {
    inner: &'static T,
}

impl<T: SerialDevice> fmt::Write for SerialOutput<T> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &byte in s.as_bytes() {
            if byte == b'\n' {
                self.inner.send(b'\r').ok();
            }
            // TODO check for errors
            self.inner.send(byte).ok();
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

    SerialOutput {
        inner: machine::console(),
    }
    .write_fmt(args)
    .ok();
}
