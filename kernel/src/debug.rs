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
use libsys::{debug::TraceLevel, error::Errno};
use core::convert::TryFrom;
use core::fmt;

pub static LEVEL: Level = Level::Debug;

/// Kernel logging levels
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u32)]
pub enum Level {
    /// Debugging information
    Debug = 1,
    /// General informational messages
    Info = 2,
    /// Non-critical warnings
    Warn = 3,
    /// Critical errors
    Error = 4,
}

impl TryFrom<u32> for Level {
    type Error = Errno;

    #[inline(always)]
    fn try_from(l: u32) -> Result<Level, Errno> {
        match l {
            1 => Ok(Level::Debug),
            2 => Ok(Level::Info),
            3 => Ok(Level::Warn),
            4 => Ok(Level::Error),
            _ => Err(Errno::InvalidArgument)
        }
    }
}

impl From<TraceLevel> for Level {
    #[inline(always)]
    fn from(l: TraceLevel) -> Self {
        match l {
            TraceLevel::Debug => Self::Debug,
            TraceLevel::Info => Self::Info,
            TraceLevel::Warn => Self::Warn,
            TraceLevel::Error => Self::Error,
        }
    }
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
pub fn _debug(level: Level, args: fmt::Arguments) {
    use crate::arch::machine;
    use fmt::Write;

    if level >= LEVEL {
        SerialOutput {
            inner: machine::console(),
        }
        .write_fmt(args)
        .ok();
    }
}
