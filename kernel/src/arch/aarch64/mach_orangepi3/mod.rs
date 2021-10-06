//! Xunlong Orange Pi 3, with Allwinner H6 SoC

use crate::arch::aarch64::timer::GenericTimer;
use crate::dev::{
    gpio::{GpioDevice, PinConfig},
    serial::SerialDevice,
    timer::TimestampSource,
    Device,
};
use crate::sync::Spin;
use error::Errno;

mod gpio;
mod uart;

use gpio::Gpio;
use uart::Uart;

#[allow(missing_docs)]
pub fn init_board() -> Result<(), Errno> {
    unsafe {
        let mut gpioh = GPIOH.lock();
        gpioh.set_pin_config(0, &PinConfig::alt(gpio::PH0_UART0_TX))?;
        gpioh.set_pin_config(1, &PinConfig::alt(gpio::PH1_UART0_RX))?;

        UART0.lock().enable()?;
    }
    Ok(())
}

const UART0_BASE: usize = 0x05000000;
const PIO_BASE: usize = 0x0300B000;

/// Returns primary console for this machine
#[inline]
pub fn console() -> &'static Spin<impl SerialDevice> {
    &UART0
}

/// Returns the timer used as CPU-local periodic IRQ source
#[inline]
pub fn local_timer() -> &'static Spin<impl TimestampSource> {
    &LOCAL_TIMER
}

static UART0: Spin<Uart> = Spin::new(unsafe { Uart::new(UART0_BASE) });
static LOCAL_TIMER: Spin<GenericTimer> = Spin::new(GenericTimer {});
#[allow(dead_code)]
static GPIOD: Spin<Gpio> = Spin::new(unsafe { Gpio::new(PIO_BASE + 0x24 * 3) });
static GPIOH: Spin<Gpio> = Spin::new(unsafe { Gpio::new(PIO_BASE + 0x24 * 7) });
