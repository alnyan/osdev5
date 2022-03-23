use crate::mem::virt::table::{MapAttributes, Entry};
use core::arch::asm;
use libsys::error::Errno;

mod table;
mod fixed;
pub use table::{EntryImpl, SpaceImpl};
use fixed::KERNEL_FIXED;

bitflags! {
    pub struct RawAttributesImpl: u64 {
        const PRESENT = EntryImpl::PRESENT;
        const WRITE = EntryImpl::WRITE;
        const USER = EntryImpl::USER;
        const BLOCK = EntryImpl::BLOCK;
        const GLOBAL = 1 << 8;
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

pub unsafe fn enable() {
    // Remove the lower mapping
    KERNEL_FIXED.pml4[0] = EntryImpl::EMPTY;

    // Flush the TLB by reloading cr3
    asm!("mov %cr3, %rax; mov %rax, %cr3", options(att_syntax));
}

pub fn map_device_memory(phys: usize, count: usize) -> Result<usize, Errno> {
    todo!()
}
