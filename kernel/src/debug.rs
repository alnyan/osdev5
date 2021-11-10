//! Debug output module.
//!
//! The module provides [print!] and [println!] macros
//! which can be used in similar way to print! and
//! println! from std.
//!
//! Level-specific debugging macros are provided as well:
//!
//! * [debugln!]
//! * [infoln!]
//! * [warnln!]
//! * [errorln!]

use crate::dev::serial::SerialDevice;
use crate::sync::IrqSafeSpinLock;
use core::fmt;

/// Kernel logging levels
#[derive(Clone, Copy, PartialEq)]
pub enum Level {
    /// Debugging information
    Debug,
    /// General informational messages
    Info,
    /// Non-critical warnings
    Warn,
    /// Critical errors
    Error,
}

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

/// Writes a formatted message to output stream
#[macro_export]
macro_rules! print {
    ($level:expr, $($it:tt)+) => ($crate::debug::_debug($level, format_args!($($it)+)))
}

/// Writes a formatted message, followed by a newline, to output stream
#[macro_export]
macro_rules! println {
    ($level:expr, $($it:tt)+) => (print!($level, "{}\n", format_args!($($it)+)))
}

/// Writes a message, annotated with current file and line, with a newline, to
/// debug level output.
///
/// See [println!].
#[macro_export]
macro_rules! debugln {
    ($($it:tt)+) => (
        print!($crate::debug::Level::Debug, "[{}:{}] {}\n", file!(), line!(), format_args!($($it)+))
    )
}

/// Writes a message, annotated with current file and line, with a newline, to
/// info level output.
///
/// See [println!].
#[macro_export]
macro_rules! infoln {
    ($($it:tt)+) => (
        print!($crate::debug::Level::Info, "\x1B[1m[{}:{}] {}\x1B[0m\n", file!(), line!(), format_args!($($it)+))
    )
}

/// Writes a message, annotated with current file and line, with a newline, to
/// warning level output.
///
/// See [println!].
#[macro_export]
macro_rules! warnln {
    ($($it:tt)+) => (
        print!($crate::debug::Level::Warn, "\x1B[33;1m[{}:{}] {}\x1B[0m\n", file!(), line!(), format_args!($($it)+))
    )
}

/// Writes a message, annotated with current file and line, with a newline, to
/// error level output.
///
/// See [println!].
#[macro_export]
macro_rules! errorln {
    ($($it:tt)+) => (
        print!($crate::debug::Level::Error, "\x1B[41;1m[{}:{}] {}\x1B[0m\n", file!(), line!(), format_args!($($it)+))
    )
}

#[doc(hidden)]
pub fn _debug(_level: Level, args: fmt::Arguments) {
    static LOCK: IrqSafeSpinLock<()> = IrqSafeSpinLock::new(());
    use crate::arch::machine;
    use fmt::Write;

    let _lock = LOCK.lock();
    SerialOutput {
        inner: machine::console(),
    }
    .write_fmt(args)
    .ok();
}
