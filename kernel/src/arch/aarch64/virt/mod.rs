//! AArch64 virtual memory management implementation

use crate::mem::virt::table::MapAttributes;
use cortex_a::{
    asm::barrier::{self, isb, dsb},
    registers::TTBR0_EL1
};
use tock_registers::interfaces::Writeable;

mod fixed;
mod table;

pub use fixed::{init_device_map, map_device_memory};
pub use table::{EntryImpl, SpaceImpl};

bitflags! {
    /// Raw attributes for AArch64 [Entry] implementation
    pub struct RawAttributesImpl: u64 {
        // TODO use 2 lower bits to determine mapping size?
        /// nG bit -- determines whether a TLB entry associated with this mapping
        ///           applies only to current ASID or all ASIDs.
        const NOT_GLOBAL = 1 << 11;
        /// AF bit -- must be set by software, otherwise Access Error exception is
        ///           generated when the page is accessed
        const ACCESS = 1 << 10;
        /// The memory region is outer-shareable
        const SH_OUTER = 2 << 8;
        /// This page is used for device-MMIO mapping and uses MAIR attribute #1
        const DEVICE = 1 << 2;

        /// Pages marked with this bit are Copy-on-Write
        const EX_COW = 1 << 55;

        /// UXN bit -- if set, page may not be used for instruction fetching from EL0
        const UXN = 1 << 54;
        /// PXN bit -- if set, page may not be used for instruction fetching from EL1
        const PXN = 1 << 53;

        // AP field
        // Default behavior is: read-write for EL1, no access for EL0
        /// If set, the page referred to by this entry is read-only for both EL0/EL1
        const AP_BOTH_READONLY = 3 << 6;
        /// If set, the page referred to by this entry is read-write for both EL0/EL1
        const AP_BOTH_READWRITE = 1 << 6;
    }
}

impl From<MapAttributes> for RawAttributesImpl {
    fn from(src: MapAttributes) -> Self {
        let mut res = RawAttributesImpl::empty();

        if src.contains(MapAttributes::SHARE_OUTER) {
            res |= RawAttributesImpl::SH_OUTER;
        }

        if !src.contains(MapAttributes::GLOBAL) {
            res |= RawAttributesImpl::NOT_GLOBAL;
        }

        if !src.contains(MapAttributes::USER_EXEC) {
            res |= RawAttributesImpl::UXN;
        }

        if !src.contains(MapAttributes::KERNEL_EXEC) {
            res |= RawAttributesImpl::PXN;
        }

        if src.contains(MapAttributes::USER_READ) {
            if src.contains(MapAttributes::USER_WRITE) {
                res |= RawAttributesImpl::AP_BOTH_READWRITE;
            } else {
                res |= RawAttributesImpl::AP_BOTH_READONLY;
            }
        }

        if src.contains(MapAttributes::DEVICE_MEMORY) {
            res |= RawAttributesImpl::DEVICE;
        }

        if src.contains(MapAttributes::ACCESS) {
            res |= RawAttributesImpl::ACCESS;
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
    fixed::init_device_map();

    dsb(barrier::ISH);
    isb(barrier::SY);

    // Disable lower-half translation
    TTBR0_EL1.set(0);
    //TCR_EL1.modify(TCR_EL1::EPD0::SET);
}
