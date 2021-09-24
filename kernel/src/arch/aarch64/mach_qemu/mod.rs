//! QEMU virt machine

use crate::arch::aarch64::timer::GenericTimer;
use crate::dev::serial::{pl011::Pl011, SerialDevice};
use crate::dev::timer::TimestampSource;
use crate::sync::Spin;

const UART0_BASE: usize = 0x09000000;

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
