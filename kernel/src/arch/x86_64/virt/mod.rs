//! x86_64 virtual memory management implementation
use crate::mem::virt::table::{MapAttributes, Entry};
use core::arch::asm;
use libsys::error::Errno;

mod table;
mod fixed;
pub use table::{EntryImpl, SpaceImpl};
use fixed::KERNEL_FIXED;

bitflags! {
    /// Raw attributes for x86_64 [Entry] implementation
    pub struct RawAttributesImpl: u64 {
        /// Entry is valid and mapped
        const PRESENT = EntryImpl::PRESENT;
        /// Entry is writable by user processes
        const WRITE = EntryImpl::WRITE;
        /// Entry is accessible (readable) by user processes
        const USER = EntryImpl::USER;
        /// Entry points to a block instead of a next-level table
        const BLOCK = EntryImpl::BLOCK;
        /// Entry is global across virtual address spaces
        const GLOBAL = 1 << 8;
        /// Entry is marked as Copy-on-Write
        const EX_COW = EntryImpl::EX_COW;
    }
}

impl From<MapAttributes> for RawAttributesImpl {
    fn from(i: MapAttributes) -> Self {
        let mut res = RawAttributesImpl::empty();

        if i.contains(MapAttributes::USER_READ) {
            res |= RawAttributesImpl::USER;
        }
        if i.contains(MapAttributes::USER_WRITE) {
            res |= RawAttributesImpl::WRITE | RawAttributesImpl::USER;
        }

        res
    }
}

/// Performs initialization of virtual memory control by kernel
///
/// # Safety
///
/// Only safe to be called once during virtual memory init.
pub unsafe fn enable() {
    // Remove the lower mapping
    KERNEL_FIXED.pml4[0] = EntryImpl::EMPTY;

    // Flush the TLB by reloading cr3
    asm!("mov %cr3, %rax; mov %rax, %cr3", options(att_syntax));
}

/// Allocates a range of virtual memory of requested size and maps
/// it to specified device memory
pub fn map_device_memory(_phys: usize, _count: usize) -> Result<usize, Errno> {
    todo!()
}
