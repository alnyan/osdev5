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
use crate::fs::devfs::{self, CharDeviceType};
use crate::mem::phys;
use syscall::error::Errno;

mod gpio;
mod rtc;
mod uart;
mod wdog;

pub use gic::IrqNumber;
use gpio::Gpio;
pub use gpio::PinAddress;
use rtc::Rtc;
use uart::Uart;
use wdog::RWdog;

pub fn init_board_early() -> Result<(), Errno> {
    unsafe {
        UART0.enable()?;

        phys::init_from_region(0x80000000, 0x10000000);
    }
    Ok(())
}

pub fn init_board() -> Result<(), Errno> {
    unsafe {
        GIC.enable()?;
        GPIO.enable()?;

        UART0.init_irqs()?;
        devfs::add_char_device(&UART0, CharDeviceType::TtySerial)?;

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

const LOCAL_TIMER_IRQ: IrqNumber = IrqNumber::new(30);
const R_WDOG_BASE: usize = 0x07020400;
const UART0_BASE: usize = 0x05000000;
const RTC_BASE: usize = 0x07000000;
const RTC_IRQ: IrqNumber = IrqNumber::new(133);
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
pub fn local_timer() -> &'static GenericTimer {
    &LOCAL_TIMER
}

/// Returns CPU's interrupt controller device
#[inline]
pub fn intc() -> &'static impl IntController<IrqNumber = IrqNumber> {
    &GIC
}

static R_WDOG: RWdog = unsafe { RWdog::new(R_WDOG_BASE) };
static UART0: Uart = unsafe { Uart::new(UART0_BASE, IrqNumber::new(32)) };
static LOCAL_TIMER: GenericTimer = GenericTimer::new(LOCAL_TIMER_IRQ);
pub(super) static GPIO: Gpio = unsafe { Gpio::new(PIO_BASE) };
static RTC: Rtc = unsafe { Rtc::new(RTC_BASE, RTC_IRQ) };
static GIC: Gic = unsafe { Gic::new(GICD_BASE, GICC_BASE, LOCAL_TIMER_IRQ) };
