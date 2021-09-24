use crate::dev::serial::{pl011::Pl011, SerialDevice};
use crate::sync::Spin;

pub const UART0_BASE: usize = 0x09000000;

#[inline]
pub fn console() -> &'static Spin<impl SerialDevice> {
    &UART0
}

static UART0: Spin<Pl011> = Spin::new(unsafe { Pl011::new(UART0_BASE) });
