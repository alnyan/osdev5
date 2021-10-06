//! QEMU virt machine

use crate::arch::aarch64::timer::GenericTimer;
use crate::dev::{Device, serial::{pl011::Pl011, SerialDevice}};
use crate::dev::timer::TimestampSource;
use crate::sync::Spin;
use error::Errno;

const UART0_BASE: usize = 0x09000000;

#[allow(missing_docs)]
pub fn init_board() -> Result<(), Errno> {
    unsafe {
        UART0.lock().enable()?;
    }
    Ok(())
}

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

static UART0: Spin<Pl011> = Spin::new(unsafe { Pl011::new(UART0_BASE) });
static LOCAL_TIMER: Spin<GenericTimer> = Spin::new(GenericTimer {});
