//! QEMU virt machine

use crate::dev::serial::{pl011::Pl011, SerialDevice};
use crate::sync::Spin;

const UART0_BASE: usize = 0x09000000;

/// Returns primary console for this machine
#[inline]
pub fn console() -> &'static Spin<impl SerialDevice> {
    &UART0
}

static UART0: Spin<Pl011> = Spin::new(unsafe { Pl011::new(UART0_BASE) });
