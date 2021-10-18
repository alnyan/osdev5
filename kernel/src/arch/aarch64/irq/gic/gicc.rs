use crate::dev::irq::IrqContext;
use crate::mem::virt::DeviceMemoryIo;
use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::registers::ReadWrite;
use tock_registers::{register_bitfields, register_structs};

register_bitfields! {
    u32,
    CTLR [
        Enable OFFSET(0) NUMBITS(1) []
    ],
    PMR [
        Priority OFFSET(0) NUMBITS(8) []
    ],
    IAR [
        InterruptID OFFSET(0) NUMBITS(10) []
    ],
    EOIR [
        EOINTID OFFSET(0) NUMBITS(10) []
    ]
}

register_structs! {
    #[allow(non_snake_case)]
    pub(super) GiccRegs {
        (0x00 => CTLR: ReadWrite<u32, CTLR::Register>),
        (0x04 => PMR: ReadWrite<u32, PMR::Register>),
        (0x08 => _res0),
        (0x0C => IAR: ReadWrite<u32, IAR::Register>),
        (0x10 => EOIR: ReadWrite<u32, EOIR::Register>),
        (0x14 => @END),
    }
}

pub(super) struct Gicc {
    regs: DeviceMemoryIo<GiccRegs>,
}

impl Gicc {
    pub const unsafe fn new(regs: DeviceMemoryIo<GiccRegs>) -> Self {
        Self { regs }
    }

    pub unsafe fn enable(&self) {
        debugln!("Enable GICC");
        self.regs.CTLR.write(CTLR::Enable::SET);
        self.regs.PMR.write(PMR::Priority.val(0xFF));
    }

    pub fn pending_irq_number<'q>(&'q self, _ic: &IrqContext<'q>) -> usize {
        self.regs.IAR.read(IAR::InterruptID) as usize
    }

    pub fn clear_irq<'q>(&'q self, irq: u32, _ic: &IrqContext<'q>) {
        self.regs.EOIR.write(EOIR::EOINTID.val(irq));
    }
}
