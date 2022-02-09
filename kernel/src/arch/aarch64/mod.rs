//! aarch64 architecture implementation

use cortex_a::registers::DAIF;
use tock_registers::interfaces::{Readable, Writeable};

pub mod boot;
pub mod context;
pub mod exception;
pub mod intrin;
pub mod irq;
pub mod reg;
pub mod timer;

cfg_if! {
    if #[cfg(feature = "mach_qemu")] {
        pub mod mach_qemu;

        pub use mach_qemu as machine;
    } else if #[cfg(feature = "mach_orangepi3")] {
        pub mod mach_orangepi3;

        pub use mach_orangepi3 as machine;
    } else if #[cfg(feature = "mach_rpi3")] {
        pub mod mach_rpi3;

        pub use mach_rpi3 as machine;
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
    intrin::irq_disable();
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
