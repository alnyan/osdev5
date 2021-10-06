use crate::arch::MemoryIo;
use crate::dev::{
    gpio::{GpioDevice, PinConfig, PinMode, PullMode},
    Device,
};
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
    regs: MemoryIo<Regs>,
}

impl Device for Gpio {
    fn name() -> &'static str {
        "Allwinner H6 GPIO Controller"
    }

    unsafe fn enable(&mut self) -> Result<(), Errno> {
        Ok(())
    }
}

impl GpioDevice for Gpio {
    unsafe fn set_pin_config(&mut self, pin: u32, cfg: &PinConfig) -> Result<(), Errno> {
        let pull = match cfg.pull {
            PullMode::None => 0,
            PullMode::Up => 1,
            PullMode::Down => 2,
        };

        match cfg.mode {
            PinMode::Disable => self.set_pin_cfg_inner(pin, 7),
            PinMode::Input => {
                self.set_pin_cfg_inner(pin, 0);
                self.set_pin_pul_inner(pin, pull);
            }
            PinMode::Output => {
                self.set_pin_cfg_inner(pin, 1); // TODO is it the same for all pins?
                self.set_pin_pul_inner(pin, pull);
            }
            PinMode::InputInterrupt => {
                todo!()
            }
            PinMode::Alt => {
                assert!(cfg.func > 1 && cfg.func < 7);
                self.set_pin_cfg_inner(pin, cfg.func);
            }
        }
        Ok(())
    }

    unsafe fn get_pin_config(&mut self, _pin: u32) -> Result<PinConfig, Errno> {
        todo!()
    }

    fn set_pin(&mut self, pin: u32) {
        self.regs.DAT.set(self.regs.DAT.get() | (1 << pin));
    }

    fn clear_pin(&mut self, pin: u32) {
        self.regs.DAT.set(self.regs.DAT.get() & !(1 << pin));
    }

    fn toggle_pin(&mut self, pin: u32) {
        self.regs.DAT.set(self.regs.DAT.get() ^ (1 << pin));
    }

    fn read_pin(&mut self, pin: u32) -> Result<bool, Errno> {
        Ok(self.regs.DAT.get() & (1 << pin) != 0)
    }
}

impl Gpio {
    #[inline]
    fn set_pin_cfg_inner(&mut self, pin: u32, cfg: u32) {
        let reg = pin >> 3;
        let shift = (pin & 0x7) * 4;
        let tmp = self.regs.CFG[reg as usize].get() & !(0xF << shift);
        self.regs.CFG[reg as usize].set(tmp | ((cfg & 0x7) << shift));
    }

    #[inline]
    fn set_pin_pul_inner(&mut self, pin: u32, pul: u32) {
        let reg = pin >> 4;
        let shift = (pin & 0xF) * 2;
        let tmp = self.regs.PUL[reg as usize].get() & !(0x3 << shift);
        self.regs.PUL[reg as usize].set(tmp | ((pul & 0x3) << shift));
    }

    pub const unsafe fn new(base: usize) -> Self {
        Self {
            regs: MemoryIo::new(base),
        }
    }
}
