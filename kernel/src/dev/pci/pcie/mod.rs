//! PCI Express access interfaces and drivers

use crate::dev::pci::{PciAddress, PciCfgSpace};

pub mod gpex;

/// Enhanced configuration space from PCI Express
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct EcamCfgSpace {
    base: usize,
}

impl EcamCfgSpace {
    /// Constructs an instance of ECAM struct describing PCI endpoint `addr`.
    ///
    /// # Safety
    ///
    /// `ecam_base` is not validated.
    pub const unsafe fn new(ecam_base: usize, addr: PciAddress) -> Self {
        Self {
            base: ecam_base + (addr.value as usize) * 4096,
        }
    }
}

impl PciCfgSpace for EcamCfgSpace {
    #[inline(always)]
    fn readl(&self, off: usize) -> u32 {
        assert!(off & 0x3 == 0);
        unsafe { core::ptr::read_volatile((self.base + off) as *const u32) }
    }

    #[inline(always)]
    unsafe fn writel(&self, off: usize, val: u32) {
        assert!(off & 0x3 == 0);
        core::ptr::write_volatile((self.base + off) as *mut u32, val);
    }
}
