#![feature(asm, const_panic)]
#![no_std]

#[macro_use]
extern crate bitflags;

pub mod abi;
pub mod debug;
pub mod error;
pub mod ioctl;
pub mod mem;
pub mod path;
pub mod proc;
pub mod signal;
pub mod stat;
pub mod termios;
pub mod traits;

#[derive(Debug)]
pub struct ProgramArgs {
    pub argv: usize,
    pub argc: usize,
    pub storage: usize,
    pub size: usize
}

// TODO utils
use core::fmt;

#[derive(Clone, Copy)]
pub struct FixedStr<const N: usize> {
    len: usize,
    data: [u8; N],
}

impl<const N: usize> FixedStr<N> {
    pub const fn empty() -> Self {
        Self {
            len: 0,
            data: [0; N]
        }
    }

    pub fn copy_from_str(&mut self, src: &str) {
        if src.len() > self.data.len() {
            panic!("copy_from_str: src len > data len");
        }
        self.len = src.len();
        self.data[..self.len].copy_from_slice(src.as_bytes());
    }

    pub fn as_str(&self) -> &str {
        unsafe {
            core::str::from_utf8_unchecked(&self.data[..self.len])
        }
    }
}

impl<const N: usize> fmt::Debug for FixedStr<N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\"")?;
        fmt::Display::fmt(self, f)?;
        write!(f, "\"")
    }
}

impl<const N: usize> fmt::Display for FixedStr<N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for &byte in &self.data[..self.len] {
            write!(f, "{}", byte as char)?;
        }
        Ok(())
    }
}

#[cfg(feature = "user")]
pub mod calls;
#[cfg(feature = "user")]
pub use calls::*;
