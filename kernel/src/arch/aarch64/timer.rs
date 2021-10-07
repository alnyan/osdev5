//! ARM generic timer implementation

use crate::dev::{timer::TimestampSource, Device};
use core::time::Duration;
use cortex_a::registers::{CNTFRQ_EL0, CNTPCT_EL0, CNTP_CTL_EL0};
use error::Errno;
use tock_registers::interfaces::{Readable, Writeable};

/// Generic timer struct
pub struct GenericTimer;

impl Device for GenericTimer {
    fn name(&self) -> &'static str {
        "ARM Generic Timer"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        CNTP_CTL_EL0.write(CNTP_CTL_EL0::ENABLE::SET);
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
