use core::fmt;
use crate::io::Write;

#[macro_export]
macro_rules! print {
    ($($args:tt)+) => ($crate::io::_print($crate::io::stdout, format_args!($($args)+)))
}

#[macro_export]
macro_rules! println {
    ($($args:tt)+) => (print!("{}\n", format_args!($($args)+)))
}

#[macro_export]
macro_rules! eprint {
    ($($args:tt)+) => ($crate::io::_print($crate::io::stderr, format_args!($($args)+)))
}

#[macro_export]
macro_rules! eprintln {
    ($($args:tt)+) => (eprint!("{}\n", format_args!($($args)+)))
}

pub fn _print<T: Write>(out: fn() -> T, args: fmt::Arguments) {
    out().write_fmt(args).expect("stdout/stderr write failed");
}
