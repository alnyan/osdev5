use crate::dev::serial::SerialDevice;

mod uart;
use uart::Uart;
mod io;
pub(self) use io::PortIo;

pub mod boot;
pub mod virt;
pub mod intrin;
pub(self) mod gdt;
pub(self) mod idt;
pub(self) mod exception;

/// Masks IRQs and returns previous IRQ mask state
///
/// # Safety
///
/// Unsafe: disables IRQ handling temporarily
#[inline(always)]
pub unsafe fn irq_mask_save() -> u64 {
    loop {}
}

/// Restores IRQ mask state
///
/// # Safety
///
/// Unsafe: modifies interrupt behavior. Must only be used in
/// conjunction with [irq_mask_save]
#[inline(always)]
pub unsafe fn irq_restore(state: u64) {
    loop {}
}

pub fn console() -> &'static impl SerialDevice {
    &COM1
}

static COM1: Uart = unsafe { Uart::new(0x3F8) };
