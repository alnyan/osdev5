use crate::dev::Device;
use crate::mem::virt::DeviceMemoryIo;
use crate::sync::IrqSafeSpinLock;
use crate::util::InitOnce;
use error::Errno;
use tock_registers::{
    interfaces::Writeable, register_bitfields, register_structs, registers::ReadWrite,
};

register_bitfields! {
    u32,
    CTRL [
        KEY OFFSET(1) NUMBITS(12) [
            Value = 0xA57
        ],
        RESTART OFFSET(0) NUMBITS(1) []
    ],
    CFG [
        CONFIG OFFSET(0) NUMBITS(2) [
            System = 1
        ]
    ],
    MODE [
        EN OFFSET(0) NUMBITS(1) []
    ]
}

register_structs! {
    #[allow(non_snake_case)]
    RWdogRegs {
        (0x00 => IRQ_EN: ReadWrite<u32>),
        (0x04 => IRQ_STA: ReadWrite<u32>),
        (0x08 => _res0),
        (0x10 => CTRL: ReadWrite<u32, CTRL::Register>),
        (0x14 => CFG: ReadWrite<u32, CFG::Register>),
        (0x18 => MODE: ReadWrite<u32, MODE::Register>),
        (0x1C => @END),
    }
}

pub(super) struct RWdog {
    inner: InitOnce<IrqSafeSpinLock<DeviceMemoryIo<RWdogRegs>>>,
    base: usize,
}

impl Device for RWdog {
    fn name(&self) -> &'static str {
        "Allwinner H6 R_WDOG"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        self.inner.init(IrqSafeSpinLock::new(DeviceMemoryIo::map(
            self.name(),
            self.base,
            1,
        )?));
        Ok(())
    }
}

impl RWdog {
    /// Performs board reset
    ///
    /// # Safety
    ///
    /// Unsafe: may interrupt critical processes
    pub unsafe fn reset_board(&self) -> ! {
        let regs = self.inner.get().lock();

        regs.CFG.write(CFG::CONFIG::System);
        regs.MODE.write(MODE::EN::SET);
        regs.CTRL.write(CTRL::KEY::Value + CTRL::RESTART::SET);

        loop {
            asm!("wfe");
        }
    }

    /// Constructs an instance of R_WDOG peripheral.
    ///
    /// # Safety
    ///
    /// Does not perform `base` validation.
    pub const unsafe fn new(base: usize) -> Self {
        Self {
            inner: InitOnce::new(),
            base,
        }
    }
}
