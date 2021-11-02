#![feature(asm)]
#![no_std]

#[cfg(feature = "linux_compat")]
compile_error!("Not yet implemented");

pub mod abi;
pub use abi::*;

#[cfg(feature = "user")]
mod calls;
#[cfg(feature = "user")]
pub use calls::*;
