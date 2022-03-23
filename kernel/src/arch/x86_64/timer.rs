//! i.... timer implementation

use crate::arch::machine::{self, IrqNumber, PortIo};
use crate::dev::{
    irq::{IntController, IntSource},
    pseudo,
    timer::TimestampSource,
    Device,
};
use crate::proc;
use core::sync::atomic::{AtomicU64, Ordering};
use core::time::Duration;
use libsys::error::Errno;

// 1.1931816666 MHz base freq

/// Generic timer struct
pub struct Timer {
    data0: PortIo<u8>,
    counter: AtomicU64,
    irq: IrqNumber,
}

impl Device for Timer {
    fn name(&self) -> &'static str {
        "Intel Timer"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        const DIV: u16 = (1193182u32 / 1000) as u16;

        self.data0.write((DIV & 0xFF) as u8);
        self.data0.write((DIV >> 8) as u8);
        Ok(())
    }
}

impl TimestampSource for Timer {
    fn timestamp(&self) -> Result<Duration, Errno> {
        Ok(Duration::from_millis(self.counter.load(Ordering::Relaxed)))
    }
}

impl IntSource for Timer {
    fn handle_irq(&self) -> Result<(), Errno> {
        let value = self.counter.fetch_add(1, Ordering::Relaxed);
        proc::wait::tick();
        pseudo::RANDOM.set_state(value as u32);
        proc::switch();
        Ok(())
    }

    fn init_irqs(&'static self) -> Result<(), Errno> {
        machine::INTC.register_handler(self.irq, self)?;
        machine::INTC.enable_irq(self.irq)?;
        Ok(())
    }
}

impl Timer {
    /// Constructs a new instance of ARM Generic Timer
    pub const fn new(irq: IrqNumber) -> Self {
        Self {
            data0: unsafe { PortIo::new(0x40) },
            counter: AtomicU64::new(0),
            irq,
        }
    }
}
