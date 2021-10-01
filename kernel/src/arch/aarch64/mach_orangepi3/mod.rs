//! QEMU virt machine

use crate::arch::aarch64::timer::GenericTimer;
use crate::dev::{Device, serial::SerialDevice};
use crate::dev::timer::TimestampSource;
use crate::sync::Spin;
use error::Errno;

fn delay(mut p: usize) {
    while p != 0 {
        cortex_a::asm::nop();
        p -= 1;
    }
}

struct Uart {
    base: usize
}

impl Device for Uart {
    fn name() -> &'static str {
        "Allwinner H6 UART"
    }

    unsafe fn enable(&mut self) -> Result<(), Errno> {
        todo!()
    }
}

impl SerialDevice for Uart {
    fn send(&mut self, byte: u8) -> Result<(), Errno> {
        unsafe {
            if byte == b'\n' {
                core::ptr::write_volatile(self.base as *mut u32, 13u32);
                delay(10000);
            }
            core::ptr::write_volatile(self.base as *mut u32, byte as u32);
            delay(10000);
        }
        Ok(())
    }

    fn recv(&mut self, blocking: bool) -> Result<u8, Errno> {
        todo!()
    }
}

const UART0_BASE: usize = 0x05000000;

/// Returns primary console for this machine
#[inline]
pub fn console() -> &'static Spin<impl SerialDevice> {
    &UART0
}

///// Returns the timer used as CPU-local periodic IRQ source
//#[inline]
//pub fn local_timer() -> &'static Spin<impl TimestampSource> {
//    &LOCAL_TIMER
//}

static UART0: Spin<Uart> = Spin::new(Uart { base: UART0_BASE });
