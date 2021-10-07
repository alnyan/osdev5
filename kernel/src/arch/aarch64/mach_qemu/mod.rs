//! QEMU virt machine

use crate::arch::aarch64::{
    irq::gic::{self, Gic},
    timer::GenericTimer,
};
use crate::dev::timer::TimestampSource;
use crate::dev::{
    irq::{IntController, IntSource},
    serial::{pl011::Pl011, SerialDevice},
    Device,
};
use error::Errno;

pub use gic::IrqNumber;

const UART0_BASE: usize = 0x09000000;
const GICD_BASE: usize = 0x08000000;
const GICC_BASE: usize = 0x08010000;

#[allow(missing_docs)]
pub fn init_board() -> Result<(), Errno> {
    unsafe {
        GIC.enable()?;

        UART0.enable()?;
        UART0.init_irqs()?;
    }
    Ok(())
}

/// Returns primary console for this machine
#[inline]
pub fn console() -> &'static impl SerialDevice {
    &UART0
}

/// Returns the timer used as CPU-local periodic IRQ source
#[inline]
pub fn local_timer() -> &'static impl TimestampSource {
    &LOCAL_TIMER
}

/// Returns CPU's interrupt controller device
#[inline]
pub fn intc() -> &'static impl IntController<IrqNumber = IrqNumber> {
    &GIC
}

static UART0: Pl011 = unsafe { Pl011::new(UART0_BASE, IrqNumber::new(33)) };
static GIC: Gic = unsafe { Gic::new(GICD_BASE, GICC_BASE) };
static LOCAL_TIMER: GenericTimer = GenericTimer {};
