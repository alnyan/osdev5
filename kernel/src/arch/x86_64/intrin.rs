//! x86_64-specific assembly functions
use core::arch::asm;

/// Disables delievery of IRQs
///
/// # Safety
///
/// Unsafe: requires ring 0
#[inline(always)]
pub unsafe fn irq_disable() {
    todo!()
}

/// Discards an entry related to `addr` from TLB cache
///
/// # Safety
///
/// Unsafe: requires ring 0
#[inline(always)]
pub unsafe fn flush_tlb_virt(addr: usize) {
    todo!()
}

/// Discards all entries related to `asid` from TLB cache
///
/// # Safety
///
/// Only safe to use for known [Process]es and their ASIDs
// TODO actually implement this on x86-64
#[inline(always)]
pub unsafe fn flush_tlb_asid(asid: usize) {
    todo!()
}
