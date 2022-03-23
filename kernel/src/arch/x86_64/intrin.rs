//! x86_64-specific assembly functions
use core::arch::asm;

/// Disables delievery of IRQs
///
/// # Safety
///
/// Unsafe: requires ring 0
#[inline(always)]
pub unsafe fn irq_disable() {
    asm!("cli");
}

/// Discards an entry related to `addr` from TLB cache
///
/// # Safety
///
/// Unsafe: requires ring 0
#[inline(always)]
pub unsafe fn flush_tlb_virt(addr: usize) {
    asm!("invlpg ({})", in(reg) addr, options(att_syntax));
}

/// Discards all entries related to `asid` from TLB cache
///
/// # Safety
///
/// Only safe to use for known [Process]es and their ASIDs
// TODO actually implement this on x86-64
#[inline(always)]
pub unsafe fn flush_tlb_asid(asid: usize) {}

#[inline(always)]
pub unsafe fn rdmsr(a: u32) -> u64 {
    let mut eax: u32;
    let mut edx: u32;
    asm!("rdmsr", in("ecx") a, out("eax") eax, out("edx") edx);
    (eax as u64) | ((edx as u64) << 32)
}

#[inline(always)]
pub unsafe fn wrmsr(a: u32, b: u64) {
    let eax = b as u32;
    let edx = (b >> 32) as u32;
    asm!("wrmsr", in("ecx") a, in("eax") eax, in("edx") edx);
}
