//! ARM generic timer implementation

use crate::dev::{
    irq::{IntController, IntSource},
    timer::TimestampSource,
    Device,
};
use crate::arch::machine::{self, IrqNumber};
use core::time::Duration;
use cortex_a::registers::{CNTFRQ_EL0, CNTP_TVAL_EL0, CNTPCT_EL0, CNTP_CTL_EL0};
use error::Errno;
use tock_registers::interfaces::{Readable, Writeable};

/// Generic timer struct
pub struct GenericTimer {
    irq: IrqNumber
}

impl Device for GenericTimer {
    fn name(&self) -> &'static str {
        "ARM Generic Timer"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        CNTP_CTL_EL0.write(CNTP_CTL_EL0::ENABLE::SET);
        Ok(())
    }
}

impl IntSource for GenericTimer {
    fn handle_irq(&self) -> Result<(), Errno> {
        CNTP_TVAL_EL0.set(10000);
        use crate::proc;
        proc::switch();
        Ok(())
    }

    fn init_irqs(&'static self) -> Result<(), Errno> {
        machine::intc().register_handler(self.irq, self)?;
        machine::intc().enable_irq(self.irq)?;
        Ok(())
    }
}

impl TimestampSource for GenericTimer {
    fn timestamp(&self) -> Result<Duration, Errno> {
        let cnt = CNTPCT_EL0.get() * 1_000_000_000;
        let frq = CNTFRQ_EL0.get();
        Ok(Duration::from_nanos(cnt / frq))
    }
}

impl GenericTimer {
    /// Constructs a new instance of ARM Generic Timer
    pub const fn new(irq: IrqNumber) -> Self {
        Self {
            irq
        }
    }
}
