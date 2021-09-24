use crate::arch::MemoryIo;
use crate::dev::{serial::SerialDevice, Device};
use error::Errno;
use tock_registers::{
    interfaces::{Readable, Writeable},
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite, WriteOnly},
};

#[repr(transparent)]
pub struct Pl011 {
    regs: MemoryIo<Regs>,
}

register_bitfields! {
    u32,
    FR [
        TXFF OFFSET(5) NUMBITS(1) [],
        RXFE OFFSET(4) NUMBITS(1) [],
        BUSY OFFSET(3) NUMBITS(1) [],
    ],
    CR [
        RXE OFFSET(9) NUMBITS(1) [],
        TXE OFFSET(8) NUMBITS(1) [],
        UARTEN OFFSET(0) NUMBITS(1) [],
    ],
    ICR [
        ALL OFFSET(0) NUMBITS(11) []
    ]
}

register_structs! {
    #[allow(non_snake_case)]
    Regs {
        (0x00 => DR: ReadWrite<u32>),
        (0x04 => _res1),
        (0x18 => FR: ReadOnly<u32, FR::Register>),
        (0x2C => LCR_H: ReadWrite<u32>),
        (0x30 => CR: ReadWrite<u32, CR::Register>),
        (0x44 => ICR: WriteOnly<u32, ICR::Register>),
        (0x04 => @END),
    }
}

impl SerialDevice for Pl011 {
    unsafe fn send(&mut self, byte: u8) -> Result<(), Errno> {
        while self.regs.FR.matches_all(FR::TXFF::SET) {
            core::hint::spin_loop();
        }
        self.regs.DR.set(byte as u32);
        Ok(())
    }

    unsafe fn recv(&mut self, blocking: bool) -> Result<u8, Errno> {
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
    pub const unsafe fn new(base: usize) -> Self {
        Self {
            regs: MemoryIo::new(base),
        }
    }
}
