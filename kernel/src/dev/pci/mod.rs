#![allow(missing_docs)]

use crate::dev::Device;
use core::fmt;
use error::Errno;

pub mod pcie;

macro_rules! ecam_field {
    ($getter:ident, $off:expr, u16) => {
        #[inline(always)]
        fn $getter(&self) -> u16 {
            self.readw($off)
        }
    };
    ($getter:ident, $off:expr, u8) => {
        #[inline(always)]
        fn $getter(&self) -> u8 {
            self.readb($off)
        }
    };
    ($getter:ident, $setter:ident, $off:expr, u16) => {
        #[inline(always)]
        unsafe fn $setter(&self, v: u16) {
            self.writew($off, v)
        }

        ecam_field! { $getter, $off, u16 }
    };
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PciAddress {
    value: u32,
}

pub trait PciCfgSpace {
    fn readl(&self, off: usize) -> u32;
    unsafe fn writel(&self, off: usize, val: u32);

    #[inline(always)]
    fn readw(&self, off: usize) -> u16 {
        assert!(off & 0x1 == 0);
        (self.readl(off & !0x3) >> ((off & 0x3) * 8)) as u16
    }

    #[inline(always)]
    fn readb(&self, off: usize) -> u8 {
        (self.readl(off & !0x3) >> ((off & 0x3) * 8)) as u8
    }

    ecam_field! { vendor_id, 0x00, u16 }
    ecam_field! { device_id, 0x02, u16 }
    ecam_field! { header_type, 0x0E, u8 }

    #[inline(always)]
    fn is_valid(&self) -> bool {
        self.readl(0) != 0xFFFFFFFF
    }
}

pub trait PciHostDevice: Device {
    fn map(&self) -> Result<(), Errno>;
}

impl PciAddress {
    #[inline(always)]
    pub const fn new(bus: u8, dev: u8, func: u8) -> Self {
        Self {
            value: ((bus as u32) << 8) | ((dev as u32) << 3) | (func as u32),
        }
    }

    #[inline(always)]
    pub const fn bus(self) -> u8 {
        (self.value >> 8) as u8
    }

    #[inline(always)]
    pub const fn dev(self) -> u8 {
        ((self.value >> 3) as u8) & 0x1F
    }

    #[inline(always)]
    pub const fn func(self) -> u8 {
        (self.value as u8) & 0x7
    }

    #[inline(always)]
    pub const fn with_func(self, func: u8) -> Self {
        Self::new(self.bus(), self.dev(), func)
    }
}

impl fmt::Debug for PciAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}",
            self.bus(),
            self.dev(),
            self.func()
        )
    }
}
