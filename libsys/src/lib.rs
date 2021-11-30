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

#[cfg(feature = "user")]
pub mod calls;
#[cfg(feature = "user")]
pub use calls::*;
