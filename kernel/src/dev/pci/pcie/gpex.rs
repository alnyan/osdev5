use crate::dev::{
    pci::{pcie::EcamCfgSpace, PciCfgSpace, PciAddress, PciHostDevice},
    Device,
};
use error::Errno;

pub struct GenericPcieHost {
    ecam_base: usize,
    // TODO
    #[allow(dead_code)]
    bus_count: u8,
}

impl Device for GenericPcieHost {
    fn name(&self) -> &'static str {
        "Generic PCIe Host Controller"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        Ok(())
    }
}

impl PciHostDevice for GenericPcieHost {
    fn map(&self) -> Result<(), Errno> {
        let bus0 = unsafe { EcamCfgSpace::new(self.ecam_base, PciAddress::new(0, 0, 0)) };

        if bus0.header_type() & 0x80 == 0 {
            self.map_bus(0)?;
        } else {
            todo!()
        }

        Ok(())
    }
}

impl GenericPcieHost {
    fn map_device(&self, addr: PciAddress) -> Result<(), Errno> {
        let fn0 = unsafe { EcamCfgSpace::new(self.ecam_base, addr) };
        if !fn0.is_valid() {
            return Ok(());
        }
        let ty = fn0.header_type();

        //self.map_function(addr, fn0)?;

        // Check if device is a multi-function one
        if ty & 0x80 != 0 {
            for func in 1..8 {
                let addr = addr.with_func(func);
                let f = unsafe { EcamCfgSpace::new(self.ecam_base, addr) };
                if f.is_valid() {
                    //self.map_function(addr, f)?;
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

    pub const unsafe fn new(ecam_base: usize, bus_count: u8) -> Self {
        Self {
            ecam_base,
            bus_count,
        }
    }
}
