//! Type-safe wrappers for different address kinds
#![no_std]
#![feature(
    step_trait,
    const_fn_trait_bound,
    const_trait_impl,
    const_panic
)]

#[cfg(test)]
#[macro_use]
extern crate std;

pub mod virt;
#[deny(missing_docs)]
pub mod phys;

trait Address {}

pub use phys::PhysicalAddress;
pub use virt::{AddressSpace, NoTrivialConvert, TrivialConvert, VirtualAddress};
