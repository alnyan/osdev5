use crate::arch::x86_64::{self, IrqNumber, PortIo};
use crate::dev::{
    irq::{IntController, IntSource},
    serial::SerialDevice,
    tty::{CharRing, TtyDevice},
    Device,
};
use libsys::error::Errno;

#[derive(TtyCharDevice)]
pub(super) struct Uart {
    dr: PortIo<u8>,
    ier: PortIo<u8>,
    isr: PortIo<u8>,
    ring: CharRing<16>,
    irq: IrqNumber,
}

impl Device for Uart {
    fn name(&self) -> &'static str {
        "x86 COM-port"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        Ok(())
    }
}

impl TtyDevice<16> for Uart {
    fn ring(&self) -> &CharRing<16> {
        &self.ring
    }
}

impl IntSource for Uart {
    fn handle_irq(&self) -> Result<(), Errno> {
        if self.isr.read() != 0 {
            self.recv_byte(self.dr.read());
        }
        Ok(())
    }

    fn init_irqs(&'static self) -> Result<(), Errno> {
        // TODO shared IRQs between COM# ports
        x86_64::INTC.register_handler(self.irq, self)?;
        self.ier.write((1 << 0));
        x86_64::INTC.enable_irq(self.irq)?;

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
    pub const unsafe fn new(base: u16, irq: IrqNumber) -> Self {
        Self {
            dr: PortIo::new(base),
            ier: PortIo::new(base + 1),
            isr: PortIo::new(base + 2),
            ring: CharRing::new(),
            irq,
        }
    }
}
