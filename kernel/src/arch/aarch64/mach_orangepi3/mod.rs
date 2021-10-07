//! Xunlong Orange Pi 3, with Allwinner H6 SoC

use crate::arch::aarch64::{
    irq::gic::{self, Gic},
    timer::GenericTimer,
};
use crate::dev::{
    gpio::{GpioDevice, PinConfig},
    irq::{IntController, IntSource},
    serial::SerialDevice,
    timer::TimestampSource,
    Device,
};
use error::Errno;

mod gpio;
mod uart;

pub use gic::IrqNumber;
use gpio::Gpio;
use uart::Uart;

#[allow(missing_docs)]
pub fn init_board() -> Result<(), Errno> {
    unsafe {
        GIC.enable()?;

        GPIOH.set_pin_config(0, &PinConfig::alt(gpio::PH0_UART0_TX))?;
        GPIOH.set_pin_config(1, &PinConfig::alt(gpio::PH1_UART0_RX))?;

        UART0.enable()?;
        UART0.init_irqs()?;
    }
    Ok(())
}

const UART0_BASE: usize = 0x05000000;
const PIO_BASE: usize = 0x0300B000;
const GICD_BASE: usize = 0x03021000;
const GICC_BASE: usize = 0x03022000;

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

///
#[inline]
pub fn intc() -> &'static impl IntController<IrqNumber = IrqNumber> {
    &GIC
}

static UART0: Uart = unsafe { Uart::new(UART0_BASE, IrqNumber::new(32)) };
static LOCAL_TIMER: GenericTimer = GenericTimer {};
#[allow(dead_code)]
static GPIOD: Gpio = unsafe { Gpio::new(PIO_BASE + 0x24 * 3) };
static GPIOH: Gpio = unsafe { Gpio::new(PIO_BASE + 0x24 * 7) };
static GIC: Gic = unsafe { Gic::new(GICD_BASE, GICC_BASE) };
