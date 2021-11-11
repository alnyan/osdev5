use crate::dev::Device;
use crate::mem::{self, virt::DeviceMemoryIo};
use crate::sync::IrqSafeSpinLock;
use crate::util::InitOnce;
use syscall::error::Errno;

use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::registers::{ReadOnly, WriteOnly};
use tock_registers::{register_bitfields, register_structs};

register_bitfields! {
    u32,
    STATUS [
        FULL OFFSET(31) NUMBITS(1) [],
        EMPTY OFFSET(30) NUMBITS(1) [],
    ],
}

register_structs! {
    #[allow(non_snake_case)]
    Regs {
        (0x00 => READ: ReadOnly<u32>),
        (0x04 => _res0),
        (0x18 => STATUS: ReadOnly<u32, STATUS::Register>),
        (0x1C => _res1),
        (0x20 => WRITE: WriteOnly<u32>),
        (0x24 => @END),
    }
}

#[repr(C, align(16))]
struct MboxBuffer([u32; 36]);

struct Inner {
    regs: DeviceMemoryIo<Regs>,
    buf: MboxBuffer,
}

pub struct Bcm283xMailbox {
    inner: InitOnce<IrqSafeSpinLock<Inner>>,
    base: usize,
}

impl Inner {
    const RESPONSE: u32 = 1 << 31;
    const REQUEST: u32 = 0;

    const PROP_ARM_MEMORY: u32 = 0x10005;
    const PROP_SET_POWER_STATE: u32 = 0x28001;
    const PROP_GET_CLOCK_RATE: u32 = 0x30002;

    fn call(&self, ch: u8) -> Result<(), Errno> {
        let ptr_virt = &self.buf as *const _ as usize;
        let ptr_phys = ptr_virt - mem::KERNEL_OFFSET;
        assert!(ptr_phys < 0x100000000);

        while self.regs.STATUS.matches_all(STATUS::FULL::SET) {
            cortex_a::asm::nop();
        }

        let val = (ptr_phys as u32) | (ch as u32);
        self.regs.WRITE.set(val);

        loop {
            while self.regs.STATUS.matches_all(STATUS::EMPTY::SET) {
                cortex_a::asm::nop();
            }

            if self.regs.READ.get() == val {
                return Ok(());
            }
        }
    }

    fn memory_split(&mut self) -> Result<usize, Errno> {
        self.buf.0[0] = 8 * 4;
        self.buf.0[1] = Self::REQUEST;

        self.buf.0[2] = Self::PROP_ARM_MEMORY;
        self.buf.0[3] = 8;
        self.buf.0[4] = 0;
        self.buf.0[5] = 0x12345678;
        self.buf.0[6] = 0x87654321;
        self.buf.0[7] = 0;

        self.call(8)?;

        if self.buf.0[1] != Self::RESPONSE {
            return Err(Errno::InvalidArgument);
        }

        Ok(self.buf.0[6] as usize)
    }

    fn set_power_state(&mut self, dev: u32, cmd: u32) -> Result<(), Errno> {
        self.buf.0[0] = 8 * 4;
        self.buf.0[1] = Self::REQUEST;

        self.buf.0[2] = Self::PROP_SET_POWER_STATE;
        self.buf.0[3] = 8;
        self.buf.0[4] = 0;
        self.buf.0[5] = dev;
        self.buf.0[6] = cmd;
        self.buf.0[7] = 0;

        self.call(8)?;

        if self.buf.0[1] != Self::RESPONSE {
            return Err(Errno::InvalidArgument);
        }

        if self.buf.0[6] & 1 << 1 != 0 {
            return Err(Errno::DoesNotExist);
        }

        if self.buf.0[6] & 1 << 0 == 0 {
            return Err(Errno::InvalidArgument);
        }

        Ok(())
    }

    fn clock_rate(&mut self, clk: u32) -> Result<u32, Errno> {
        self.buf.0[0] = 8 * 4;
        self.buf.0[1] = Self::REQUEST;

        self.buf.0[2] = Self::PROP_GET_CLOCK_RATE;
        self.buf.0[3] = 8;
        self.buf.0[4] = 0;
        self.buf.0[5] = clk;
        self.buf.0[6] = 0;
        self.buf.0[7] = 0;

        self.call(8)?;

        if self.buf.0[1] != Self::RESPONSE {
            return Err(Errno::InvalidArgument);
        }

        Ok(self.buf.0[6])
    }
}

impl Device for Bcm283xMailbox {
    fn name(&self) -> &'static str {
        "BCM283x Mailbox"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        self.inner.init(IrqSafeSpinLock::new(Inner {
            regs: DeviceMemoryIo::map(self.name(), self.base, 1)?,
            buf: MboxBuffer([0; 36]),
        }));

        Ok(())
    }
}

impl Bcm283xMailbox {
    pub const POWER_STATE_ON: u32 = 1 << 0;
    pub const POWER_STATE_WAIT: u32 = 1 << 1;
    pub const POWER_SD_CARD: u32 = 0;

    pub const CLOCK_EMMC: u32 = 1;

    pub fn memory_split(&self) -> Result<usize, Errno> {
        self.inner.get().lock().memory_split()
    }

    pub fn set_power_state(&self, dev: u32, cmd: u32) -> Result<(), Errno> {
        self.inner.get().lock().set_power_state(dev, cmd)
    }

    pub fn clock_rate(&self, clk: u32) -> Result<u32, Errno> {
        self.inner.get().lock().clock_rate(clk)
    }

    pub const unsafe fn new(base: usize) -> Self {
        Self {
            inner: InitOnce::new(),
            base,
        }
    }
}
