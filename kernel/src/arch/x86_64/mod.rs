use crate::dev::{serial::SerialDevice, display::StaticFramebuffer, irq::IntController};
use core::arch::asm;

mod uart;
use uart::Uart;
mod intc;
use intc::I8259;

mod io;
pub(self) use io::PortIo;

pub mod boot;
pub mod virt;
pub mod intrin;
pub mod context;
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
    let mut res;
    asm!("pushf; cli; pop {}", out(reg) res, options(att_syntax));
    res
}

/// Restores IRQ mask state
///
/// # Safety
///
/// Unsafe: modifies interrupt behavior. Must only be used in
/// conjunction with [irq_mask_save]
#[inline(always)]
pub unsafe fn irq_restore(state: u64) {
    if state & (1 << 9) != 0 {
        asm!("sti");
    }
}

pub fn intc() -> &'static impl IntController {
    &INTC
}

pub fn console() -> &'static impl SerialDevice {
    &COM1
}

static COM1: Uart = unsafe { Uart::new(0x3F8) };
static INTC: I8259 = I8259::new();
pub(self) static DISPLAY: StaticFramebuffer = StaticFramebuffer::uninit();
