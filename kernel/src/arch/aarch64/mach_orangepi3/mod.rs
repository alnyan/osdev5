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
mod rtc;
mod wdog;

pub use gic::IrqNumber;
pub use gpio::PinAddress;
use gpio::Gpio;
use uart::Uart;
use rtc::Rtc;
use wdog::RWdog;

#[allow(missing_docs)]
pub fn init_board() -> Result<(), Errno> {
    unsafe {
        UART0.enable()?;
        GIC.enable()?;
        GPIO.enable()?;

        UART0.init_irqs()?;

        R_WDOG.enable()?;

        GPIO.cfg_uart0_ph0_ph1()?;
        GPIO.set_pin_config(PinAddress::new(3, 26), &PinConfig::out_pull_down())?;

        RTC.enable()?;
        RTC.init_irqs()?;
    }
    Ok(())
}

/// Performs board reset
///
/// # Safety
///
/// Unsafe: may interrupt critical processes
pub unsafe fn reset_board() -> ! {
    R_WDOG.reset_board()
}

const R_WDOG_BASE: usize = 0x07020400;
const UART0_BASE: usize = 0x05000000;
const RTC_BASE: usize = 0x07000000;
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

/// Returns CPU's interrupt controller device
#[inline]
pub fn intc() -> &'static impl IntController<IrqNumber = IrqNumber> {
    &GIC
}

static R_WDOG: RWdog = unsafe { RWdog::new(R_WDOG_BASE) };
static UART0: Uart = unsafe { Uart::new(UART0_BASE, IrqNumber::new(32)) };
static LOCAL_TIMER: GenericTimer = GenericTimer {};
pub(super) static GPIO: Gpio = unsafe { Gpio::new(PIO_BASE) };
static RTC: Rtc = unsafe { Rtc::new(RTC_BASE, IrqNumber::new(133)) };
static GIC: Gic = unsafe { Gic::new(GICD_BASE, GICC_BASE) };
