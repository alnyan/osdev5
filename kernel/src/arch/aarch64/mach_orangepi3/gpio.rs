//! Allwinner H6 GPIO port controller driver.
//!
//! GPIO ports are split into two register groups:
//!
//! 1. CPUS-PORT (TODO PL, PM)
//! 2. CPUX-PORT (PC, PD, PF, PG, PH)
//!
use crate::dev::{
    gpio::{GpioDevice, PinConfig, PinMode, PullMode},
    Device,
};
use crate::mem::virt::DeviceMemoryIo;
use crate::sync::IrqSafeSpinLock;
use crate::util::InitOnce;
use syscall::error::Errno;
use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::register_structs;
use tock_registers::registers::ReadWrite;

register_structs! {
    #[allow(non_snake_case)]
    CpuxPortRegs {
        (0x00 => CFG: [ReadWrite<u32>; 4]),
        (0x10 => DAT: ReadWrite<u32>),
        (0x14 => DRV: [ReadWrite<u32>; 2]),
        (0x1C => PUL: [ReadWrite<u32>; 2]),
        (0x24 => @END),
    }
}

struct CpuxGpio {
    regs: DeviceMemoryIo<[CpuxPortRegs; 8]>,
}

pub struct Gpio {
    cpux: InitOnce<IrqSafeSpinLock<CpuxGpio>>,
    cpux_base: usize,
}

/// Structure combining bank and pin numbers
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PinAddress(u32);

impl PinAddress {
    /// Constructs a new pin address from `bank` and `pin` numbers
    #[inline(always)]
    pub const fn new(bank: u32, pin: u32) -> Self {
        // TODO sanity checks
        Self((bank << 16) | pin)
    }

    /// Returns bank number of this pin
    #[inline(always)]
    pub const fn bank(self) -> usize {
        (self.0 >> 16) as usize
    }

    /// Returns pin number of this pin
    #[inline(always)]
    pub const fn pin(self) -> u32 {
        self.0 & 0xFFFF
    }
}

impl CpuxPortRegs {
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

impl CpuxGpio {
    unsafe fn set_pin_config(&self, bank: usize, pin: u32, cfg: &PinConfig) -> Result<(), Errno> {
        let regs = &self.regs[bank];

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

    #[inline(always)]
    fn read_pin(&self, bank: usize, pin: u32) -> bool {
        self.regs[bank].DAT.get() & (1u32 << pin) != 0
    }

    #[inline(always)]
    fn toggle_pin(&mut self, bank: usize, pin: u32) {
        self.regs[bank]
            .DAT
            .set(self.regs[bank].DAT.get() ^ (1u32 << pin))
    }

    #[inline(always)]
    fn write_pin(&mut self, bank: usize, pin: u32, value: bool) {
        if value {
            self.regs[bank]
                .DAT
                .set(self.regs[bank].DAT.get() | (1u32 << pin))
        } else {
            self.regs[bank]
                .DAT
                .set(self.regs[bank].DAT.get() & !(1u32 << pin))
        }
    }
}

impl Device for Gpio {
    fn name(&self) -> &'static str {
        "Allwinner H6 GPIO Controller"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        self.cpux.init(IrqSafeSpinLock::new(CpuxGpio {
            regs: DeviceMemoryIo::map(self.name(), self.cpux_base, 1)?,
        }));
        Ok(())
    }
}

impl GpioDevice for Gpio {
    type PinAddress = PinAddress;

    unsafe fn set_pin_config(&self, pin: PinAddress, cfg: &PinConfig) -> Result<(), Errno> {
        let bank = pin.bank();
        let pin = pin.pin();

        match bank {
            0 | 1 | 4 => unimplemented!(),
            _ => self.cpux.get().lock().set_pin_config(bank, pin, cfg),
        }
    }

    fn get_pin_config(&self, _pin: PinAddress) -> Result<PinConfig, Errno> {
        todo!()
    }

    fn write_pin(&self, pin: PinAddress, state: bool) {
        let bank = pin.bank();
        let pin = pin.pin();

        match bank {
            0 | 1 | 4 => unimplemented!(),
            _ => self.cpux.get().lock().write_pin(bank, pin, state),
        }
    }

    fn toggle_pin(&self, pin: PinAddress) {
        let bank = pin.bank();
        let pin = pin.pin();

        match bank {
            0 | 1 | 4 => unimplemented!(),
            _ => self.cpux.get().lock().toggle_pin(bank, pin),
        }
    }

    fn read_pin(&self, pin: PinAddress) -> Result<bool, Errno> {
        let bank = pin.bank();
        let pin = pin.pin();

        match bank {
            0 | 1 | 4 => unimplemented!(),
            _ => Ok(self.cpux.get().lock().read_pin(bank, pin)),
        }
    }
}

impl Gpio {
    pub unsafe fn cfg_uart0_ph0_ph1(&self) -> Result<(), Errno> {
        self.set_pin_config(PinAddress::new(7, 0), &PinConfig::alt(2))?;
        self.set_pin_config(PinAddress::new(7, 1), &PinConfig::alt(2))
    }

    pub const unsafe fn new(cpux_base: usize) -> Self {
        Self {
            cpux: InitOnce::new(),
            cpux_base,
        }
    }
}
