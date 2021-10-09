//! ARM Generic Interrupt Controller

use crate::dev::{
    irq::{IntController, IntSource, IrqContext},
    Device,
};
use crate::mem::virt::DeviceMemoryIo;
use crate::sync::IrqSafeNullLock;
use crate::util::InitOnce;
use error::Errno;

mod gicc;
use gicc::Gicc;
mod gicd;
use gicd::Gicd;

/// Maximum available IRQ number
pub const MAX_IRQ: usize = 300;

/// Range-checked IRQ number type
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct IrqNumber(usize);

/// ARM Generic Interrupt Controller, version 2
pub struct Gic {
    gicc: InitOnce<Gicc>,
    gicd: InitOnce<Gicd>,
    gicd_base: usize,
    gicc_base: usize,
    table: IrqSafeNullLock<[Option<&'static (dyn IntSource + Sync)>; MAX_IRQ]>,
}

impl IrqNumber {
    /// Returns numeric representation for given [IrqNumber]
    #[inline(always)]
    pub const fn get(self) -> usize {
        self.0
    }

    /// Checks and wraps an IRQ number
    #[inline(always)]
    pub const fn new(v: usize) -> Self {
        assert!(v < MAX_IRQ);
        Self(v)
    }
}

impl Device for Gic {
    fn name(&self) -> &'static str {
        "ARM Generic Interrupt Controller"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        let gicd_mmio_shared =
            DeviceMemoryIo::map("GICv2 shared Distributor registers", self.gicd_base, 1)?;
        let gicd_mmio_banked =
            DeviceMemoryIo::map("GICv2 banked Distributor registers", self.gicd_base, 1)?;
        let gicc_mmio = DeviceMemoryIo::map("GICv2 CPU registers", self.gicc_base, 1)?;

        let mut gicd = Gicd::new(gicd_mmio_shared, gicd_mmio_banked);
        let mut gicc = Gicc::new(gicc_mmio);

        gicd.enable();
        gicc.enable();

        self.gicd.init(gicd);
        self.gicc.init(gicc);

        Ok(())
    }
}

impl IntController for Gic {
    type IrqNumber = IrqNumber;

    fn enable_irq(&self, irq: Self::IrqNumber) -> Result<(), Errno> {
        self.gicd.get().enable_irq(irq);
        Ok(())
    }

    fn handle_pending_irqs<'irq_context>(&'irq_context self, ic: &IrqContext<'irq_context>) {
        let gicc = self.gicc.get();
        let irq_number = gicc.pending_irq_number(ic);
        if irq_number >= MAX_IRQ {
            return;
        }

        {
            let table = self.table.lock();
            match table[irq_number] {
                None => panic!("No handler registered for irq{}", irq_number),
                Some(handler) => handler.handle_irq().expect("irq handler failed"),
            }
        }

        gicc.clear_irq(irq_number as u32, ic);
    }

    fn register_handler(
        &self,
        irq: Self::IrqNumber,
        handler: &'static (dyn IntSource + Sync),
    ) -> Result<(), Errno> {
        let mut table = self.table.lock();
        let irq = irq.get();
        if table[irq].is_some() {
            return Err(Errno::AlreadyExists);
        }

        debugln!("Bound irq{} to {:?}", irq, Device::name(handler));
        table[irq] = Some(handler);

        Ok(())
    }
}

impl Gic {
    /// Constructs an instance of GICv2.
    ///
    /// # Safety
    ///
    /// Does not perform `gicd_base` and `gicc_base` validation.
    pub const unsafe fn new(gicd_base: usize, gicc_base: usize) -> Self {
        Self {
            gicc: InitOnce::new(),
            gicd: InitOnce::new(),
            gicd_base,
            gicc_base,
            table: IrqSafeNullLock::new([None; MAX_IRQ]),
        }
    }
}
