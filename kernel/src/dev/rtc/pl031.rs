//! PL031 - ARM PrimeCell real-time clock implementation
use crate::dev::{Device, rtc::RtcDevice, irq::{IntController, IntSource}};
use crate::arch::{MemoryIo, machine::{self, IrqNumber}};
use crate::sync::IrqSafeNullLock;
use error::Errno;
use tock_registers::{
    interfaces::{Readable, Writeable, ReadWriteable},
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite, WriteOnly},
};

register_bitfields! {
    u32,
    CR [
        RTCStart OFFSET(0) NUMBITS(1) []
    ],
    IMSC [
        RTCIMSC OFFSET(0) NUMBITS(1) []
    ],
    ICR [
        RTCICR OFFSET(0) NUMBITS(1) []
    ]
}

register_structs! {
    #[allow(non_snake_case)]
    Regs {
        (0x00 => DR: ReadOnly<u32>),
        (0x04 => MR: ReadWrite<u32>),
        (0x08 => LR: ReadWrite<u32>),
        (0x0C => CR: ReadWrite<u32, CR::Register>),
        (0x10 => IMSC: ReadWrite<u32, IMSC::Register>),
        (0x14 => RIS: ReadOnly<u32>),
        (0x18 => MIS: ReadOnly<u32>),
        (0x1C => ICR: WriteOnly<u32, ICR::Register>),
        (0x20 => @END),
    }
}

/// Device struct for PL031
pub struct Pl031 {
    regs: IrqSafeNullLock<MemoryIo<Regs>>,
    irq: IrqNumber
}

impl RtcDevice for Pl031 {
}

impl IntSource for Pl031 {
    fn handle_irq(&self) -> Result<(), Errno> {
        let regs = self.regs.lock();
        regs.ICR.write(ICR::RTCICR::SET);
        let data = regs.DR.get();
        regs.MR.set(data + 1);
        Ok(())
    }

    fn init_irqs(&'static self) -> Result<(), Errno> {
        machine::intc().register_handler(self.irq, self)?;
        self.regs.lock().IMSC.modify(IMSC::RTCIMSC::SET);
        machine::intc().enable_irq(self.irq)?;

        Ok(())
    }
}

impl Device for Pl031 {
    fn name(&self) -> &'static str {
        "PL031 RTC"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        let regs = self.regs.lock();
        regs.CR.modify(CR::RTCStart::CLEAR);
        regs.MR.set(regs.DR.get() + 1);
        regs.CR.modify(CR::RTCStart::SET);
        Ok(())
    }
}

impl Pl031 {
    /// Constructs an instance of PL031 device.
    ///
    /// # Safety
    ///
    /// Does not perform `base` validation.
    pub const unsafe fn new(base: usize, irq: IrqNumber) -> Self {
        Self {
            regs: IrqSafeNullLock::new(MemoryIo::new(base)),
            irq
        }
    }
}
