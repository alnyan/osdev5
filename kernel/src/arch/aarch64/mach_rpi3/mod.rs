#![allow(missing_docs)]

use crate::arch::aarch64::timer::GenericTimer;
use crate::dev::{irq::IntController, serial::SerialDevice, timer::TimestampSource, Device};
use crate::mem::phys;
use error::Errno;

mod irqchip;
use irqchip::Bcm283xIntController;
pub use irqchip::IrqNumber;
mod uart;
use uart::Bcm283xMiniUart;

pub fn init_board_early() -> Result<(), Errno> {
    unsafe {
        MUART.enable()?;

        phys::init_from_region(0x0, 0x30000000);
    }
    Ok(())
}

pub fn init_board() -> Result<(), Errno> {
    unsafe {
        INTC.enable()?;
    }
    Ok(())
}

pub fn console() -> &'static impl SerialDevice {
    &MUART
}

/// Returns the timer used as CPU-local periodic IRQ source
#[inline]
pub fn local_timer() -> &'static impl TimestampSource {
    &LOCAL_TIMER
}

pub fn intc() -> &'static impl IntController<IrqNumber = IrqNumber> {
    &INTC
}

static INTC: Bcm283xIntController = unsafe { Bcm283xIntController::new() };
static MUART: Bcm283xMiniUart = unsafe { Bcm283xMiniUart::new(0x3F215040) };
static LOCAL_TIMER: GenericTimer = GenericTimer {};
