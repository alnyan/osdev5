use crate::dev::{serial::SerialDevice, Device};
use crate::mem::virt::DeviceMemoryIo;
use crate::sync::IrqSafeSpinLock;
use crate::util::InitOnce;
use error::Errno;
use tock_registers::{
    interfaces::{ReadWriteable, Readable, Writeable},
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite, WriteOnly},
};

register_structs! {
    Regs {
        (0x00 => IO: ReadWrite<u32>),
        (0x04 => @END),
    }
}

pub(super) struct Bcm283xMiniUart {
    regs: InitOnce<IrqSafeSpinLock<DeviceMemoryIo<Regs>>>,
    base: usize,
}

impl Device for Bcm283xMiniUart {
    fn name(&self) -> &'static str {
        "BCM283x Mini-UART"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        self.regs.init(IrqSafeSpinLock::new(DeviceMemoryIo::map(
            self.name(),
            self.base,
            1,
        )?));
        Ok(())
    }
}

impl SerialDevice for Bcm283xMiniUart {
    fn send(&self, byte: u8) -> Result<(), Errno> {
        if !self.regs.is_initialized() {
            return Ok(());
        }

        let regs = self.regs.get().lock();
        regs.IO.set(byte as u32);
        Ok(())
    }

    fn recv(&self, _blocking: bool) -> Result<u8, Errno> {
        todo!()
    }
}

impl Bcm283xMiniUart {
    /// Constructs an instance of MiniUART device.
    ///
    /// # Safety
    ///
    /// Does not perform `base` validation.
    pub const unsafe fn new(base: usize) -> Self {
        Self {
            regs: InitOnce::new(),
            base,
        }
    }
}
