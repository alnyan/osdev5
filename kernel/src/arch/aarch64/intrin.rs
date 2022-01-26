use core::arch::asm;

#[inline(always)]
pub unsafe fn irq_disable() {
    asm!("msr daifset, {bits}", bits = const 2, options(nomem, nostack, preserves_flags));
}

#[inline(always)]
pub unsafe fn flush_tlb_virt(addr: usize) {
}

// TODO non-portable
#[inline(always)]
pub unsafe fn flush_tlb_asid(asid: usize) {
    asm!("tlbi aside1, {}", in(reg) asid);
}

