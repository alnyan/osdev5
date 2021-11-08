#![feature(asm)]
#![no_std]

#[macro_use]
extern crate bitflags;

pub mod abi;
pub mod stat;
pub mod termios;

#[cfg(feature = "user")]
pub mod calls;
#[cfg(feature = "user")]
pub use calls::*;
