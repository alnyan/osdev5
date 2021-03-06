use crate::arch::machine::{self, IrqNumber};
use crate::dev::{
    irq::{IntController, IntSource},
    serial::SerialDevice,
    tty::{CharRing, TtyDevice},
    Device,
};
use crate::mem::virt::DeviceMemoryIo;
use crate::sync::IrqSafeSpinLock;
use crate::util::InitOnce;
use libsys::error::Errno;
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};
use tock_registers::registers::{Aliased, ReadOnly, ReadWrite};
use tock_registers::{register_bitfields, register_structs};

register_bitfields! [
    u32,
    IER [
        PTIME OFFSET(7) NUMBITS(1) [],
        RS485_INT_EN OFFSET(4) NUMBITS(1) [],
        EDSSI OFFSET(3) NUMBITS(1) [],
        ELSI OFFSET(2) NUMBITS(1) [],
        ETBEI OFFSET(1) NUMBITS(1) [],
        ERBFI OFFSET(0) NUMBITS(1) [],
    ],
    IIR [
        FEFLAG OFFSET(6) NUMBITS(2) [
            Enable = 3,
            Disable = 0
        ],
        IID OFFSET(0) NUMBITS(4) [
            ModemStatus = 0,
            NoInterrupt = 1,
            ThrEmpty = 2,
            Rs485Interrupt = 3,
            ReceivedDataAvailable = 4,
            ReceiverLineStatus = 6,
            BusyDetect = 7,
            CharacterTimeout = 12
        ]
    ],
    LSR [
        FIFOERR OFFSET(7) NUMBITS(1) [],
        TEMT OFFSET(6) NUMBITS(1) [],
        THRE OFFSET(5) NUMBITS(1) [],
        BI OFFSET(4) NUMBITS(1) [],
        FE OFFSET(3) NUMBITS(1) [],
        PE OFFSET(2) NUMBITS(1) [],
        OE OFFSET(1) NUMBITS(1) [],
        DR OFFSET(0) NUMBITS(1) []
    ]
];

register_structs! {
    #[allow(non_snake_case)]
    Regs {
        (0x0000 => DR_DLL: Aliased<u32>),
        (0x0004 => IER_DLH: ReadWrite<u32, IER::Register>),
        (0x0008 => IIR_FCR: Aliased<u32, IIR::Register, ()>),
        (0x000C => LCR: ReadWrite<u32>),
        (0x0010 => MCR: ReadWrite<u32>),
        (0x0014 => LSR: ReadOnly<u32, LSR::Register>),
        (0x0018 => MSR: ReadOnly<u32>),
        (0x001C => SCH: ReadWrite<u32>),
        (0x0020 => _res0),
        (0x007C => USR: ReadOnly<u32>),
        (0x0080 => TFL: ReadWrite<u32>),
        (0x0084 => RFL: ReadWrite<u32>),
        (0x0088 => HSK: ReadWrite<u32>),
        (0x008C => _res1),
        (0x00A4 => HALT: ReadWrite<u32>),
        (0x00D0 => @END),
    }
}

struct UartInner {
    regs: DeviceMemoryIo<Regs>,
}

#[derive(TtyCharDevice)]
pub(super) struct Uart {
    inner: InitOnce<IrqSafeSpinLock<UartInner>>,
    ring: CharRing<16>,
    base: usize,
    irq: IrqNumber,
}

impl Device for Uart {
    fn name(&self) -> &'static str {
        "Allwinner H6 UART"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        let mut inner = UartInner {
            regs: DeviceMemoryIo::map(self.name(), self.base, 1)?,
        };
        // TODO
        self.inner.init(IrqSafeSpinLock::new(inner));
        Ok(())
    }
}

impl SerialDevice for Uart {
    fn send(&self, byte: u8) -> Result<(), Errno> {
        if !self.inner.is_initialized() {
            return Ok(());
        }

        let inner = self.inner.get().lock();
        while !inner.regs.LSR.matches_all(LSR::THRE::SET) {
            cortex_a::asm::nop();
        }
        inner.regs.DR_DLL.set(byte as u32);
        Ok(())
    }

    fn recv(&self, _blocking: bool) -> Result<u8, Errno> {
        let inner = self.inner.get().lock();
        while !inner.regs.LSR.matches_all(LSR::DR::SET) {
            cortex_a::asm::nop();
        }
        Ok(inner.regs.DR_DLL.get() as u8)
    }
}

impl TtyDevice<16> for Uart {
    fn ring(&self) -> &CharRing<16> {
        &self.ring
    }
}

impl IntSource for Uart {
    fn handle_irq(&self) -> Result<(), Errno> {
        let byte = self.inner.get().lock().regs.DR_DLL.get();

        if byte == 0x1B {
            debugln!("Received ESC, resetting");
            unsafe {
                machine::reset_board();
            }
        }

        self.recv_byte(byte as u8);
        Ok(())
    }

    fn init_irqs(&'static self) -> Result<(), Errno> {
        machine::intc().register_handler(self.irq, self)?;
        self.inner.get().lock().regs.IER_DLH.modify(IER::ERBFI::SET);
        machine::intc().enable_irq(self.irq)?;

        Ok(())
    }
}

impl Uart {
    pub const unsafe fn new(base: usize, irq: IrqNumber) -> Self {
        Self {
            inner: InitOnce::new(),
            ring: CharRing::new(),
            base,
            irq,
        }
    }
}
