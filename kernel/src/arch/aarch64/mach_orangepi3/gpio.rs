use crate::arch::MemoryIo;
use crate::dev::{
    gpio::{GpioDevice, PinConfig, PinMode, PullMode},
    Device,
};
use crate::sync::IrqSafeNullLock;
use error::Errno;
use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::register_structs;
use tock_registers::registers::ReadWrite;

pub const PH0_UART0_TX: u32 = 2;
pub const PH1_UART0_RX: u32 = 2;

register_structs! {
    #[allow(non_snake_case)]
    Regs {
        (0x00 => CFG: [ReadWrite<u32>; 4]),
        (0x10 => DAT: ReadWrite<u32>),
        (0x14 => DRV: [ReadWrite<u32>; 2]),
        (0x1C => PUL: [ReadWrite<u32>; 2]),
        (0x24 => @END),
    }
}

pub(super) struct Gpio {
    regs: IrqSafeNullLock<MemoryIo<Regs>>,
}

impl Regs {
    #[inline]
    fn set_pin_cfg_inner(&self, pin: u32, cfg: u32) {
        let reg = pin >> 3;
        let shift = (pin & 0x7) * 4;
        let tmp = self.CFG[reg as usize].get() & !(0xF << shift);
        self.CFG[reg as usize].set(tmp | ((cfg & 0x7) << shift));
    }

    #[inline]
    fn set_pin_pul_inner(&self, pin: u32, pul: u32) {
        let reg = pin >> 4;
        let shift = (pin & 0xF) * 2;
        let tmp = self.PUL[reg as usize].get() & !(0x3 << shift);
        self.PUL[reg as usize].set(tmp | ((pul & 0x3) << shift));
    }

}

impl Device for Gpio {
    fn name(&self) -> &'static str {
        "Allwinner H6 GPIO Controller"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        Ok(())
    }
}

impl GpioDevice for Gpio {
    unsafe fn set_pin_config(&self, pin: u32, cfg: &PinConfig) -> Result<(), Errno> {
        let regs = self.regs.lock();
        let pull = match cfg.pull {
            PullMode::None => 0,
            PullMode::Up => 1,
            PullMode::Down => 2,
        };

        match cfg.mode {
            PinMode::Disable => regs.set_pin_cfg_inner(pin, 7),
            PinMode::Input => {
                regs.set_pin_cfg_inner(pin, 0);
                regs.set_pin_pul_inner(pin, pull);
            }
            PinMode::Output => {
                regs.set_pin_cfg_inner(pin, 1); // TODO is it the same for all pins?
                regs.set_pin_pul_inner(pin, pull);
            }
            PinMode::InputInterrupt => {
                todo!()
            }
            PinMode::Alt => {
                assert!(cfg.func > 1 && cfg.func < 7);
                regs.set_pin_cfg_inner(pin, cfg.func);
            }
        }
        Ok(())
    }

    unsafe fn get_pin_config(&self, _pin: u32) -> Result<PinConfig, Errno> {
        todo!()
    }

    fn set_pin(&self, pin: u32) {
        let regs = self.regs.lock();
        regs.DAT.set(regs.DAT.get() | (1 << pin));
    }

    fn clear_pin(&self, pin: u32) {
        let regs = self.regs.lock();
        regs.DAT.set(regs.DAT.get() & !(1 << pin));
    }

    fn toggle_pin(&self, pin: u32) {
        let regs = self.regs.lock();
        regs.DAT.set(regs.DAT.get() ^ (1 << pin));
    }

    fn read_pin(&self, pin: u32) -> Result<bool, Errno> {
        let regs = self.regs.lock();
        Ok(regs.DAT.get() & (1 << pin) != 0)
    }
}

impl Gpio {
    pub const unsafe fn new(base: usize) -> Self {
        Self {
            regs: IrqSafeNullLock::new(MemoryIo::new(base)),
        }
    }
}
