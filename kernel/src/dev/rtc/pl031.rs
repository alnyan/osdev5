//! PL031 - ARM PrimeCell real-time clock implementation
use crate::arch::machine::{self, IrqNumber};
use crate::dev::{
    irq::{IntController, IntSource},
    rtc::RtcDevice,
    Device,
};
use crate::mem::virt::DeviceMemoryIo;
use crate::sync::IrqSafeSpinLock;
use crate::util::InitOnce;
use libsys::error::Errno;
use tock_registers::{
    interfaces::{ReadWriteable, Readable, Writeable},
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

struct Pl031Inner {
    regs: DeviceMemoryIo<Regs>,
}

/// Device struct for PL031
pub struct Pl031 {
    inner: InitOnce<IrqSafeSpinLock<Pl031Inner>>,
    base: usize,
    irq: IrqNumber,
}

impl RtcDevice for Pl031 {}

impl IntSource for Pl031 {
    fn handle_irq(&self) -> Result<(), Errno> {
        let inner = self.inner.get().lock();
        inner.regs.ICR.write(ICR::RTCICR::SET);
        let data = inner.regs.DR.get();
        inner.regs.MR.set(data + 1);
        Ok(())
    }

    fn init_irqs(&'static self) -> Result<(), Errno> {
        machine::intc().register_handler(self.irq, self)?;
        self.inner.get().lock().regs.IMSC.modify(IMSC::RTCIMSC::SET);
        machine::intc().enable_irq(self.irq)?;

        Ok(())
    }
}

impl Device for Pl031 {
    fn name(&self) -> &'static str {
        "PL031 RTC"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        let inner = Pl031Inner {
            regs: DeviceMemoryIo::map(self.name(), self.base, 1)?,
        };

        inner.regs.CR.modify(CR::RTCStart::CLEAR);
        inner.regs.MR.set(inner.regs.DR.get() + 1);
        inner.regs.CR.modify(CR::RTCStart::SET);

        self.inner.init(IrqSafeSpinLock::new(inner));

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
            inner: InitOnce::new(),
            base,
            irq,
        }
    }
}
