use core::arch::asm;
use core::marker::PhantomData;

pub struct PortIo<T> {
    port: u16,
    _pd: PhantomData<T>,
}

impl<T> PortIo<T> {
    pub const unsafe fn new(port: u16) -> Self {
        Self {
            port,
            _pd: PhantomData,
        }
    }
}

impl PortIo<u8> {
    pub fn read(&self) -> u8 {
        let mut res: u8;
        unsafe {
            asm!("inb %dx, %al", in("dx") self.port, out("al") res, options(att_syntax));
        }
        res
    }

    pub fn write(&self, value: u8) {
        unsafe {
            asm!("outb %al, %dx", in("dx") self.port, in("al") value, options(att_syntax));
        }
    }
}
