//! PCI bus host and device interfaces

use crate::dev::Device;
use core::fmt;
use error::Errno;

pub mod pcie;

macro_rules! ecam_field {
    ($getter:ident, $off:expr, u16) => {
        #[allow(missing_docs)]
        #[inline(always)]
        fn $getter(&self) -> u16 {
            self.readw($off)
        }
    };
    ($getter:ident, $off:expr, u8) => {
        #[allow(missing_docs)]
        #[inline(always)]
        fn $getter(&self) -> u8 {
            self.readb($off)
        }
    };
    ($getter:ident, $setter:ident, $off:expr, u16) => {
        #[allow(missing_docs)]
        #[inline(always)]
        unsafe fn $setter(&self, v: u16) {
            self.writew($off, v)
        }

        ecam_field! { $getter, $off, u16 }
    };
}

/// PCI endpoint address struct, combining bus:dev:func parts
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PciAddress {
    value: u32,
}

/// Generic PCI device configuration space interface
pub trait PciCfgSpace {
    // TODO change readl to readl_unchecked() and perform checks at trait level
    /// Reads an [u32] from device config space.
    /// `off` must be aligned at a 4-byte boundary.
    fn readl(&self, off: usize) -> u32;

    /// Writes an [u32] to device config space.
    /// `off` must be aligned at a 4-byte boundary.
    ///
    /// # Safety
    ///
    /// Unsafe: allows arbitrary value writes to PCI config space.
    unsafe fn writel(&self, off: usize, val: u32);

    /// Reads an [u16] from device config space.
    /// `off` must be aligned at a 2-byte boundary.
    #[inline(always)]
    fn readw(&self, off: usize) -> u16 {
        assert!(off & 0x1 == 0);
        (self.readl(off & !0x3) >> ((off & 0x3) * 8)) as u16
    }

    /// Reads an [u8] from device config space
    #[inline(always)]
    fn readb(&self, off: usize) -> u8 {
        (self.readl(off & !0x3) >> ((off & 0x3) * 8)) as u8
    }

    ecam_field! { vendor_id, 0x00, u16 }
    ecam_field! { device_id, 0x02, u16 }
    ecam_field! { header_type, 0x0E, u8 }

    /// Returns `true` if device this config describes is
    /// present on the bus
    #[inline(always)]
    fn is_valid(&self) -> bool {
        self.readl(0) != 0xFFFFFFFF
    }
}

/// PCI host controller interface
pub trait PciHostDevice: Device {
    /// Initializes and enables devices attached to the bus
    fn map(&self) -> Result<(), Errno>;
}

impl PciAddress {
    /// Constructs a [PciAddress] instance from its components
    #[inline(always)]
    pub const fn new(bus: u8, dev: u8, func: u8) -> Self {
        Self {
            value: ((bus as u32) << 8) | ((dev as u32) << 3) | (func as u32),
        }
    }

    /// Returns `bus` field of [PciAddress]
    #[inline(always)]
    pub const fn bus(self) -> u8 {
        (self.value >> 8) as u8
    }

    /// Returns `dev` field of [PciAddress]
    #[inline(always)]
    pub const fn dev(self) -> u8 {
        ((self.value >> 3) as u8) & 0x1F
    }

    /// Returns `func` field of [PciAddress]
    #[inline(always)]
    pub const fn func(self) -> u8 {
        (self.value as u8) & 0x7
    }

    /// Returns a new [PciAddress], constructed from `self`, but with
    /// specified `func` number
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
