use crate::arch::machine::{self, IrqNumber};
use crate::dev::{
    irq::{IntController, IntSource},
    rtc::RtcDevice,
    Device,
};
use crate::mem::virt::DeviceMemoryIo;
use crate::sync::IrqSafeSpinLock;
use crate::util::InitOnce;
use syscall::error::Errno;
use tock_registers::{
    interfaces::{Readable, Writeable},
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite},
};

register_bitfields! {
    u32,
    ALARM0_IRQ_EN [
        ALARM0_IRQ_EN OFFSET(0) NUMBITS(1) []
    ],
    ALARM0_ENABLE [
        ALM_0_EN OFFSET(0) NUMBITS(1) []
    ],
    ALARM0_IRQ_STA [
        ALARM0_IRQ_PEND OFFSET(0) NUMBITS(1) []
    ]
}

register_structs! {
    #[allow(non_snake_case)]
    Regs {
        (0x00 => LOSC_CTRL: ReadWrite<u32>),
        (0x04 => LOSC_AUTO_SWT_STA: ReadWrite<u32>),
        (0x08 => INTOSC_CLK_PRESCAL: ReadWrite<u32>),
        (0x0C => INTOSC_CLK_AUTO_CALI: ReadWrite<u32>),
        (0x10 => RTC_YY_MM_DD: ReadWrite<u32>),
        (0x14 => RTC_HH_MM_SS: ReadWrite<u32>),
        (0x18 => _res0),
        (0x20 => ALARM0_COUNTER: ReadWrite<u32>),
        (0x24 => ALARM0_CUR_VLU: ReadOnly<u32>),
        (0x28 => ALARM0_ENABLE: ReadWrite<u32, ALARM0_ENABLE::Register>),
        (0x2C => ALARM0_IRQ_EN: ReadWrite<u32, ALARM0_IRQ_EN::Register>),
        (0x30 => ALARM0_IRQ_STA: ReadWrite<u32, ALARM0_IRQ_STA::Register>),
        (0x34 => @END),
    }
}

pub struct Rtc {
    regs: InitOnce<IrqSafeSpinLock<DeviceMemoryIo<Regs>>>,
    base: usize,
    irq: IrqNumber,
}

impl Regs {
    fn arm_alarm0_irq(&self, sec: u32) {
        // Clear IRQ pending status
        if sec == 0 {
            return;
        }
        self.ALARM0_IRQ_STA
            .write(ALARM0_IRQ_STA::ALARM0_IRQ_PEND::SET);
        self.ALARM0_IRQ_EN.write(ALARM0_IRQ_EN::ALARM0_IRQ_EN::SET);
        self.ALARM0_COUNTER.set(self.ALARM0_CUR_VLU.get() + sec - 1);
        self.ALARM0_ENABLE.write(ALARM0_ENABLE::ALM_0_EN::SET);
    }
}

impl RtcDevice for Rtc {}

impl IntSource for Rtc {
    fn handle_irq(&self) -> Result<(), Errno> {
        self.regs.get().lock().arm_alarm0_irq(1);
        Ok(())
    }

    fn init_irqs(&'static self) -> Result<(), Errno> {
        machine::intc().register_handler(self.irq, self)?;
        self.regs.get().lock().arm_alarm0_irq(1);
        machine::intc().enable_irq(self.irq)?;

        Ok(())
    }
}

impl Device for Rtc {
    fn name(&self) -> &'static str {
        "Allwinner H6 RTC"
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

impl Rtc {
    /// Constructs an instance of RTC device.
    ///
    /// # Safety
    ///
    /// Does not perform `base` validation.
    pub const unsafe fn new(base: usize, irq: IrqNumber) -> Self {
        Self {
            regs: InitOnce::new(),
            base,
            irq,
        }
    }
}
