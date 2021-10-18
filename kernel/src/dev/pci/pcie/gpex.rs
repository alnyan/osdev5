//! Generic PCIe host driver

use crate::dev::{
    pci::{pcie::EcamCfgSpace, PciAddress, PciCfgSpace, PciHostDevice},
    Device,
};
use crate::mem::virt::DeviceMemory;
use crate::util::InitOnce;
use error::Errno;

/// GPEX host controller struct
pub struct GenericPcieHost {
    ecam_base: usize,
    ecam: InitOnce<DeviceMemory>,
    // TODO
    #[allow(dead_code)]
    bus_count: u8,
}

impl Device for GenericPcieHost {
    fn name(&self) -> &'static str {
        "Generic PCIe Host Controller"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        self.ecam
            .init(DeviceMemory::map(self.name(), self.ecam_base, 512 * 512)?);
        Ok(())
    }
}

impl PciHostDevice for GenericPcieHost {
    fn map(&self) -> Result<(), Errno> {
        let bus0 = self.get_ecam(PciAddress::new(0, 0, 0));

        if bus0.header_type() & 0x80 == 0 {
            self.map_bus(0)?;
        } else {
            todo!()
        }

        Ok(())
    }
}

impl GenericPcieHost {
    fn get_ecam(&self, addr: PciAddress) -> EcamCfgSpace {
        assert!(addr.value < 512 * 512);
        unsafe { EcamCfgSpace::new(self.ecam.get().base(), addr) }
    }

    fn map_function(&self, addr: PciAddress, cfg: EcamCfgSpace) -> Result<(), Errno> {
        infoln!(
            "{:?}: {:04x}:{:04x}",
            addr,
            cfg.vendor_id(),
            cfg.device_id()
        );
        Ok(())
    }

    fn map_device(&self, addr: PciAddress) -> Result<(), Errno> {
        let fn0 = self.get_ecam(addr);
        if !fn0.is_valid() {
            return Ok(());
        }
        let ty = fn0.header_type();

        self.map_function(addr, fn0)?;

        // Check if device is a multi-function one
        if ty & 0x80 != 0 {
            for func in 1..8 {
                let addr = addr.with_func(func);
                let f = self.get_ecam(addr);
                if f.is_valid() {
                    self.map_function(addr, f)?;
                }
            }
        }

        Ok(())
    }

    fn map_bus(&self, bus: u8) -> Result<(), Errno> {
        for dev in 0u8..=255 {
            self.map_device(PciAddress::new(bus, dev, 0))?;
        }

        Ok(())
    }

    /// Constructs an instance of GPEX device.
    ///
    /// # Safety
    ///
    /// Does not perform `ecam_base` validation.
    pub const unsafe fn new(ecam_base: usize, bus_count: u8) -> Self {
        Self {
            ecam: InitOnce::new(),
            ecam_base,
            bus_count,
        }
    }
}
