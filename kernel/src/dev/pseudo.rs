use crate::arch::machine::{self, IrqNumber};
use crate::dev::{
    irq::{IntController, IntSource},
    serial::SerialDevice,
    tty::{CharRing, TtyDevice},
    Device,
};
use crate::mem::virt::DeviceMemoryIo;
use crate::sync::IrqSafeSpinLock;
use crate::util::InitOnce;
use libsys::{error::Errno, ioctl::IoctlCmd};
use core::sync::atomic::{AtomicU32, Ordering};
use tock_registers::{
    interfaces::{ReadWriteable, Readable, Writeable},
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite, WriteOnly},
};
use vfs::CharDevice;

pub struct Random {
    state: AtomicU32
}
pub struct Zero;

impl Device for Random {
    fn name(&self) -> &'static str {
        "Pseudo-random device"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        Ok(())
    }
}

impl CharDevice for Random {
    fn read(&self, _blocking: bool, data: &mut [u8]) -> Result<usize, Errno> {
        for byte in data.iter_mut() {
            *byte = self.read_single() as u8;
        }
        Ok(data.len())
    }

    fn write(&self, _blocking: bool, _data: &[u8]) -> Result<usize, Errno> {
        Ok(0)
    }

    fn is_ready(&self, _write: bool) -> Result<bool, Errno> {
        Ok(true)
    }

    fn ioctl(&self, _cmd: IoctlCmd, _ptr: usize, _lim: usize) -> Result<usize, Errno> {
        Err(Errno::InvalidArgument)
    }
}


impl Device for Zero {
    fn name(&self) -> &'static str {
        "Zero device"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        Ok(())
    }
}

impl CharDevice for Zero {
    fn read(&self, _blocking: bool, data: &mut [u8]) -> Result<usize, Errno> {
        data.fill(0);
        Ok(data.len())
    }

    fn write(&self, _blocking: bool, _data: &[u8]) -> Result<usize, Errno> {
        Ok(0)
    }

    fn is_ready(&self, _write: bool) -> Result<bool, Errno> {
        Ok(true)
    }

    fn ioctl(&self, _cmd: IoctlCmd, _ptr: usize, _lim: usize) -> Result<usize, Errno> {
        Err(Errno::InvalidArgument)
    }
}

impl Random {
    pub fn set_state(&self, state: u32) {
        self.state.store(state, Ordering::Release);
    }

    pub fn read_single(&self) -> u32 {
        let mut x = self.state.load(Ordering::Acquire);
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state.store(x, Ordering::Release);
        x
    }
}

pub static RANDOM: Random = Random { state: AtomicU32::new(0) };
pub static ZERO: Zero = Zero;
