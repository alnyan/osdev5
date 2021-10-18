use crate::dev::{
    irq::{IntController, IntSource, IrqContext},
    Device,
};
use error::Errno;

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct IrqNumber(u32);

pub(super) struct Bcm283xIntController {}

impl Device for Bcm283xIntController {
    fn name(&self) -> &'static str {
        "BCM283x interrupt controller"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        Ok(())
    }
}

impl IntController for Bcm283xIntController {
    type IrqNumber = IrqNumber;

    fn register_handler(
        &self,
        irq: IrqNumber,
        handler: &'static (dyn IntSource + Sync),
    ) -> Result<(), Errno> {
        todo!()
    }

    fn enable_irq(&self, irq: IrqNumber) -> Result<(), Errno> {
        todo!()
    }

    fn handle_pending_irqs<'q>(&'q self, _ic: &IrqContext<'q>) {
        todo!()
    }
}

impl Bcm283xIntController {
    pub const unsafe fn new() -> Self {
        Self {}
    }
}
