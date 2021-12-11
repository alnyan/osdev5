use crate::dev::{Device, serial::SerialDevice};
use crate::arch::x86_64::PortIo;
use libsys::error::Errno;

pub(super) struct Uart {
    dr: PortIo<u8>
}

impl Device for Uart {
    fn name(&self) -> &'static str {
        "x86 COM-port"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        Ok(())
    }
}

impl SerialDevice for Uart {
    fn send(&self, byte: u8) -> Result<(), Errno> {
        self.dr.write(byte);
        Ok(())
    }

    fn recv(&self, _blocking: bool) -> Result<u8, Errno> {
        todo!()
    }
}

impl Uart {
    pub const unsafe fn new(base: u16) -> Self {
        Self {
            dr: PortIo::new(base)
        }
    }
}
