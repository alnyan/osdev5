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

use crate::dev::{
    display::{Display, FramebufferInfo},
    serial::SerialDevice,
};
use crate::font;
use crate::sync::IrqSafeSpinLock;
use core::convert::TryFrom;
use core::fmt;
use libsys::{debug::TraceLevel, error::Errno};

/// Currently active print level
pub static LEVEL: Level = Level::Debug;
static COLOR_MAP: [u32; 16] = [
    0x000000,
    0x0000AA,
    0x00AA00,
    0x00AAAA,
    0xAA0000,
    0xAA00AA,
    0xAA5500,
    0xAAAAAA,
    0x555555,
    0x5555FF,
    0x55FF55,
    0x55FFFF,
    0xFF5555,
    0xFF55FF,
    0xFFFF55,
    0xFFFFFF,
];
static ATTR_MAP: [usize; 10] = [
     0, 4, 2, 6, 1, 5, 3, 7, 7, 7
];
static DISPLAY: IrqSafeSpinLock<FramebufferOutput> = IrqSafeSpinLock::new(FramebufferOutput {
    display: None,
    col: 0,
    row: 0,
    fg: 0xBBBBBB,
    bg: 0x000000,
    esc: EscapeState::None,
    esc_argv: [0; 8],
    esc_argc: 0
});

enum EscapeState {
    None,
    Esc,
    Data
}

struct FramebufferOutput {
    display: Option<&'static dyn Display>,
    row: usize,
    col: usize,
    fg: u32,
    bg: u32,
    esc: EscapeState,
    esc_argv: [usize; 8],
    esc_argc: usize
}

impl fmt::Write for FramebufferOutput {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.display.is_none() {
            return Ok(());
        }
        let fb = self.display.unwrap().framebuffer().unwrap();

        for ch in s.chars() {
            self.putc(&fb, ch);
        }

        Ok(())
    }
}

pub fn set_display(disp: &'static dyn Display) {
    DISPLAY.lock().display = Some(disp);
}

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
            _ => Err(Errno::InvalidArgument),
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

    if level > Level::Debug {
        DISPLAY.lock().write_fmt(args).ok();
    }

    if level >= LEVEL {
        SerialOutput {
            inner: machine::console(),
        }
        .write_fmt(args)
        .ok();
    }
}

impl FramebufferOutput {
    const CW: usize = 8;
    const CH: usize = 12;

    pub fn set_char(&mut self, fb: &FramebufferInfo, x: usize, y: usize, ch: char) {
        if (x + 1) * Self::CW >= fb.width || (y + 1) * Self::CH >= fb.height {
            return;
        }
        font::get().draw(fb, x * Self::CW, y * Self::CH, ch, self.fg, self.bg);
    }

    pub fn putc(&mut self, fb: &FramebufferInfo, ch: char) {
        match self.esc {
            EscapeState::None => {
                match ch {
                    '\x1B' => {
                        self.esc = EscapeState::Esc;
                        self.esc_argv.fill(0);
                        self.esc_argc = 0;
                    }
                    ' '..='\x7E' => {
                        self.set_char(fb, self.col, self.row, ch);

                        // Advance the cursor
                        self.col += 1;
                        if (self.col + 1) * Self::CW >= fb.width {
                            self.col = 0;
                            self.row += 1;
                        }
                    }
                    '\n' => {
                        self.col = 0;
                        self.row += 1;
                    }
                    _ => {}
                }

                if (self.row + 1) * Self::CH >= fb.height {
                    todo!()
                }
            }
            EscapeState::Esc => {
                match ch {
                    '[' => {
                        self.esc = EscapeState::Data;
                    }
                    _ => {
                        self.esc = EscapeState::None;
                    }
                }
            }
            EscapeState::Data => {
                match ch {
                    '0'..='9' => {
                        self.esc_argv[self.esc_argc] *= 10;
                        self.esc_argv[self.esc_argc] += (ch as u8 - b'0') as usize;
                    }
                    ';' => {
                        self.esc_argc += 1;
                    }
                    _ => {
                        self.esc_argc += 1;
                        self.esc = EscapeState::None;
                    }
                }

                match ch {
                    'm' => {
                        for i in 0..self.esc_argc {
                            let item = self.esc_argv[i];
                            if item / 10 == 4 {
                                self.bg = COLOR_MAP[ATTR_MAP[(item % 10) as usize]];
                            }
                            if item / 10 == 3 {
                                self.fg = COLOR_MAP[ATTR_MAP[(item % 10) as usize]];
                            }

                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
