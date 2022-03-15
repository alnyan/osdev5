//! AArch64-specific assembly functions
use core::arch::asm;

/// Disables delievery of IRQs
///
/// # Safety
///
/// Unsafe: requires EL0
#[inline(always)]
pub unsafe fn irq_disable() {
    asm!("msr daifset, {bits}", bits = const 2, options(nomem, nostack, preserves_flags));
}

/// Discards an entry related to `addr` from TLB cache
///
/// # Safety
///
/// Unsafe: requires EL0
#[inline(always)]
pub unsafe fn flush_tlb_virt(addr: usize) {
    asm!("tlbi vaae1, {}", in(reg) addr);
}

/// Discards all entries related to `asid` from TLB cache
///
/// # Safety
///
/// Only safe to use for known [Process]es and their ASIDs
// TODO non-portable
#[inline(always)]
pub unsafe fn flush_tlb_asid(asid: usize) {
    asm!("tlbi aside1, {}", in(reg) asid);
}

