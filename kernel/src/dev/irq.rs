//! Interrupt controller and handler interfaces
use crate::arch::platform::smp::NodeAddress;
use crate::dev::Device;
use core::marker::PhantomData;
use error::Errno;

/// Token to indicate the local core is running in IRQ context
pub struct IrqContext<'irq_context> {
    _0: PhantomData<&'irq_context ()>,
}

/// Interrupt controller interface
pub trait IntController: Device {
    /// Implementation-specific definition for "IRQ line"
    type IrqNumber;

    /// Binds a handler [IntSource] to a specific `irq` line
    fn register_handler(
        &self,
        irq: Self::IrqNumber,
        handler: &'static (dyn IntSource + Sync),
    ) -> Result<(), Errno>;

    /// Enables/unmasks `irq` line
    fn enable_irq(&self, irq: Self::IrqNumber) -> Result<(), Errno>;

    /// Handles all pending IRQs for this interrupt controller
    fn handle_pending_irqs<'irq_context>(&'irq_context self, ic: &IrqContext<'irq_context>);
}

/// Inter-processor interrupt delivery method
pub trait IpiSender: Device {
    /// Raise an IPI for the target CPU mask, optionally excluding source CPU
    fn send_to_mask(&self, except_self: bool, target: u32, data: u64);
}

/// Interface for peripherals capable of emitting IRQs
pub trait IntSource: Device {
    /// Handles pending IRQs, if any, of this [IntSource].
    ///
    /// If no IRQ is pending, returns [Errno::DoesNotExist]
    fn handle_irq(&self) -> Result<(), Errno>;

    ///
    fn init_irqs(&'static self) -> Result<(), Errno>;
}

impl<'q> IrqContext<'q> {
    /// Constructs an IRQ context token
    ///
    /// # Safety
    ///
    /// Only allowed to be constructed in top-level IRQ handlers
    #[inline(always)]
    pub unsafe fn new() -> Self {
        Self { _0: PhantomData }
    }
}
