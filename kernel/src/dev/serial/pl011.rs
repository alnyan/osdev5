//! PL011 - ARM PrimeCell UART implementation

use crate::arch::{MemoryIo, machine::{self, IrqNumber}};
use crate::dev::{serial::SerialDevice, irq::{IntController, IntSource}, Device};
use crate::sync::IrqSafeNullLock;
use error::Errno;
use tock_registers::{
    interfaces::{Readable, Writeable, ReadWriteable},
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite, WriteOnly},
};

register_bitfields! {
    u32,
    /// Flag register
    FR [
        /// Transmit FIFO full
        TXFF OFFSET(5) NUMBITS(1) [],
        /// Receive FIFO empty
        RXFE OFFSET(4) NUMBITS(1) [],
        /// UART busy
        BUSY OFFSET(3) NUMBITS(1) [],
    ],
    /// Control register
    CR [
        /// Enable UART receiver
        RXE OFFSET(9) NUMBITS(1) [],
        /// Enable UART transmitter
        TXE OFFSET(8) NUMBITS(1) [],
        /// Enable UART
        UARTEN OFFSET(0) NUMBITS(1) [],
    ],
    /// Interrupt clear register
    ICR [
        /// Writing this to ICR clears all IRQs
        ALL OFFSET(0) NUMBITS(11) []
    ],
    /// Interrupt mask set/clear register
    IMSC [
        RXIM OFFSET(4) NUMBITS(1) []
    ]
}

register_structs! {
    /// PL011 registers
    #[allow(non_snake_case)]
    Regs {
        /// Data register
        (0x00 => DR: ReadWrite<u32>),
        (0x04 => _res1),
        /// Flag register
        (0x18 => FR: ReadOnly<u32, FR::Register>),
        (0x1C => _res2),
        /// Line control register
        (0x2C => LCR_H: ReadWrite<u32>),
        /// Control register
        (0x30 => CR: ReadWrite<u32, CR::Register>),
        (0x34 => IFLS: ReadWrite<u32>),
        (0x38 => IMSC: ReadWrite<u32, IMSC::Register>),
        (0x3C => _res3),
        /// Interrupt clear register
        (0x44 => ICR: WriteOnly<u32, ICR::Register>),
        (0x04 => @END),
    }
}

/// Device struct for PL011
pub struct Pl011 {
    regs: IrqSafeNullLock<MemoryIo<Regs>>,
    irq: IrqNumber,
}

impl IntSource for Pl011 {
    fn handle_irq(&self) -> Result<(), Errno> {
        let regs = self.regs.lock();
        regs.ICR.write(ICR::ALL::CLEAR);

        let byte = regs.DR.get();
        debugln!("irq byte = {:#04x}", byte);


        Ok(())
    }

    fn init_irqs(&'static self) -> Result<(), Errno> {
        machine::intc().register_handler(self.irq, self)?;
        self.regs.lock().IMSC.modify(IMSC::RXIM::SET);
        machine::intc().enable_irq(self.irq)?;

        Ok(())
    }
}

impl SerialDevice for Pl011 {
    fn send(&self, byte: u8) -> Result<(), Errno> {
        let regs = self.regs.lock();
        while regs.FR.matches_all(FR::TXFF::SET) {
            core::hint::spin_loop();
        }
        regs.DR.set(byte as u32);
        Ok(())
    }

    fn recv(&self, blocking: bool) -> Result<u8, Errno> {
        let regs = self.regs.lock();
        if regs.FR.matches_all(FR::RXFE::SET) {
            if !blocking {
                return Err(Errno::WouldBlock);
            }
            while regs.FR.matches_all(FR::RXFE::SET) {
                // TODO allow IRQs here?
                core::hint::spin_loop();
            }
        }

        Ok(regs.DR.get() as u8)
    }
}

impl Device for Pl011 {
    fn name(&self) -> &'static str {
        "PL011 UART"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        let regs = self.regs.lock();
        regs.CR.set(0);
        regs.ICR.write(ICR::ALL::CLEAR);
        regs.CR.write(CR::UARTEN::SET + CR::TXE::SET + CR::RXE::SET);

        Ok(())
    }
}

impl Pl011 {
    /// Constructs an instance of PL011 device.
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
