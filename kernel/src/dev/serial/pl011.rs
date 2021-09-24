//! PL011 - ARM PrimeCell UART implementation

use crate::arch::MemoryIo;
use crate::dev::{serial::SerialDevice, Device};
use error::Errno;
use tock_registers::{
    interfaces::{Readable, Writeable},
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite, WriteOnly},
};

/// Device struct for PL011
#[repr(transparent)]
pub struct Pl011 {
    regs: MemoryIo<Regs>,
}

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
        /// Line control register
        (0x2C => LCR_H: ReadWrite<u32>),
        /// Control register
        (0x30 => CR: ReadWrite<u32, CR::Register>),
        /// Interrupt clear register
        (0x44 => ICR: WriteOnly<u32, ICR::Register>),
        (0x04 => @END),
    }
}

impl SerialDevice for Pl011 {
    fn send(&mut self, byte: u8) -> Result<(), Errno> {
        while self.regs.FR.matches_all(FR::TXFF::SET) {
            core::hint::spin_loop();
        }
        self.regs.DR.set(byte as u32);
        Ok(())
    }

    fn recv(&mut self, blocking: bool) -> Result<u8, Errno> {
        if self.regs.FR.matches_all(FR::RXFE::SET) {
            if !blocking {
                return Err(Errno::WouldBlock);
            }
            while self.regs.FR.matches_all(FR::RXFE::SET) {
                core::hint::spin_loop();
            }
        }

        Ok(self.regs.DR.get() as u8)
    }
}

impl Device for Pl011 {
    fn name() -> &'static str {
        "PL011 UART"
    }

    unsafe fn enable(&mut self) -> Result<(), Errno> {
        self.regs.CR.set(0);
        self.regs.ICR.write(ICR::ALL::CLEAR);
        self.regs
            .CR
            .write(CR::UARTEN::SET + CR::TXE::SET + CR::RXE::SET);

        Ok(())
    }
}

impl Pl011 {
    /// Constructs an instance of PL011 device.
    ///
    /// # Safety
    ///
    /// Does not perform `base` validation.
    pub const unsafe fn new(base: usize) -> Self {
        Self {
            regs: MemoryIo::new(base),
        }
    }
}
