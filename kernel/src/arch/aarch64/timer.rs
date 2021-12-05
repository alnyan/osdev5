//! ARM generic timer implementation

use crate::arch::machine::{self, IrqNumber};
use crate::dev::{
    irq::{IntController, IntSource},
    timer::TimestampSource,
    Device,
};
use core::time::Duration;
use cortex_a::registers::{CNTFRQ_EL0, CNTPCT_EL0, CNTP_CTL_EL0, CNTP_TVAL_EL0};
use libsys::error::Errno;
use tock_registers::interfaces::{Readable, Writeable};

/// Generic timer struct
pub struct GenericTimer {
    irq: IrqNumber,
}

/// Duration of a single timer period
pub const TIMER_TICK: u64 = 1000000;

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
        CNTP_TVAL_EL0.set(TIMER_TICK);
        CNTP_CTL_EL0.write(CNTP_CTL_EL0::ENABLE::SET);
        use crate::proc;
        proc::wait::tick();
        proc::switch();
        Ok(())
    }

    fn init_irqs(&'static self) -> Result<(), Errno> {
        machine::intc().register_handler(self.irq, self)?;
        CNTP_TVAL_EL0.set(TIMER_TICK);
        machine::intc().enable_irq(self.irq)?;
        Ok(())
    }
}

impl TimestampSource for GenericTimer {
    fn timestamp(&self) -> Result<Duration, Errno> {
        let cnt = (CNTPCT_EL0.get() as u128) * 1_000_000_000u128;
        let frq = CNTFRQ_EL0.get() as u128;
        let secs = ((cnt / frq) / 1_000_000_000) as u64;
        let nanos = ((cnt / frq) % 1_000_000_000) as u32;
        Ok(Duration::new(secs, nanos))
    }
}

impl GenericTimer {
    /// Constructs a new instance of ARM Generic Timer
    pub const fn new(irq: IrqNumber) -> Self {
        Self { irq }
    }
}
