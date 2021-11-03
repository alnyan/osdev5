#![feature(asm)]
#![no_std]

#[cfg(feature = "linux_compat")]
compile_error!("Not yet implemented");

#[macro_use]
extern crate bitflags;

pub mod abi;
pub mod stat;

#[cfg(feature = "user")]
pub mod calls;
#[cfg(feature = "user")]
pub use calls::*;
