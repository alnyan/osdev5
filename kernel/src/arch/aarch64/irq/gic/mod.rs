//! ARM Generic Interrupt Controller

use crate::dev::{
    irq::{IntController, IntSource, IrqContext},
    Device,
};
use crate::sync::IrqSafeNullLock;
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
    gicc: Gicc,
    gicd: Gicd,
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
        self.gicd.enable();
        self.gicc.enable();
        Ok(())
    }
}

impl IntController for Gic {
    type IrqNumber = IrqNumber;

    fn enable_irq(&self, irq: Self::IrqNumber) -> Result<(), Errno> {
        self.gicd.enable_irq(irq);
        Ok(())
    }

    fn handle_pending_irqs<'irq_context>(&'irq_context self, ic: &IrqContext<'irq_context>) {
        let irq_number = self.gicc.pending_irq_number(ic);
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

        self.gicc.clear_irq(irq_number as u32, ic);
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
            gicc: Gicc::new(gicc_base),
            gicd: Gicd::new(gicd_base),
            table: IrqSafeNullLock::new([None; MAX_IRQ]),
        }
    }
}
