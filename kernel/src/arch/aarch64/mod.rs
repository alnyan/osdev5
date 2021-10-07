//! aarch64 architecture implementation

use cortex_a::registers::DAIF;
use tock_registers::interfaces::{Readable, Writeable};

pub mod asm;
pub mod boot;
pub mod exception;
pub mod irq;
pub mod timer;

cfg_if! {
    if #[cfg(feature = "mach_qemu")] {
        pub mod mach_qemu;

        pub use mach_qemu as machine;
    } else if #[cfg(feature = "mach_orangepi3")] {
        pub mod mach_orangepi3;

        pub use mach_orangepi3 as machine;
    }
}

/// Masks IRQs and returns previous IRQ mask state
///
/// # Safety
///
/// Unsafe: disables IRQ handling temporarily
#[inline(always)]
pub unsafe fn irq_mask_save() -> u64 {
    let state = DAIF.get();
    asm!("msr daifset, {bits}", bits = const 2, options(nomem, nostack, preserves_flags));
    state
}

/// Restores IRQ mask state
///
/// # Safety
///
/// Unsafe: modifies interrupt behavior. Must only be used in
/// conjunction with [irq_mask_save]
#[inline(always)]
pub unsafe fn irq_restore(state: u64) {
    DAIF.set(state);
}
