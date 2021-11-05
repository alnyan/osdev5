use crate::{OpenFlags, Stat, VnodeImpl, VnodeKind, VnodeRef};
use error::Errno;

/// Generic character device trait
pub trait CharDevice {
    /// Performs a read from the device into [data] buffer.
    ///
    /// If no data is available and `blocking` is set, will ask
    /// the OS to suspend the calling thread until data arrives.
    /// Otherwise, will immediately return an error.
    fn read(&self, blocking: bool, data: &mut [u8]) -> Result<usize, Errno>;
    /// Performs a write to the device from [data] buffer.
    ///
    /// If the device cannot (at the moment) accept data and
    /// `blocking` is set, will block until it's available. Otherwise,
    /// will immediately return an error.
    fn write(&self, blocking: bool, data: &[u8]) -> Result<usize, Errno>;
}

/// Wrapper struct to attach [VnodeImpl] implementation
/// to [CharDevice]s
pub struct CharDeviceWrapper {
    device: &'static dyn CharDevice,
}

impl VnodeImpl for CharDeviceWrapper {
    fn create(&mut self, _at: VnodeRef, _name: &str, _kind: VnodeKind) -> Result<VnodeRef, Errno> {
        panic!();
    }

    fn remove(&mut self, _at: VnodeRef, _name: &str) -> Result<(), Errno> {
        panic!();
    }

    fn lookup(&mut self, _at: VnodeRef, _name: &str) -> Result<VnodeRef, Errno> {
        panic!();
    }

    fn open(&mut self, _node: VnodeRef, _opts: OpenFlags) -> Result<usize, Errno> {
        Ok(0)
    }

    fn close(&mut self, _node: VnodeRef) -> Result<(), Errno> {
        Ok(())
    }

    fn read(&mut self, _node: VnodeRef, _pos: usize, data: &mut [u8]) -> Result<usize, Errno> {
        self.device.read(true, data)
    }

    fn write(&mut self, _node: VnodeRef, _pos: usize, data: &[u8]) -> Result<usize, Errno> {
        self.device.write(true, data)
    }

    fn truncate(&mut self, _node: VnodeRef, _size: usize) -> Result<(), Errno> {
        panic!();
    }

    fn size(&mut self, _node: VnodeRef) -> Result<usize, Errno> {
        panic!();
    }

    fn stat(&mut self, _node: VnodeRef, _stat: &mut Stat) -> Result<(), Errno> {
        todo!();
    }
}

impl CharDeviceWrapper {
    /// Creates a wrapper for static [CharDevice] trait object to
    /// auto-implement [VnodeImpl] trait for the device
    pub const fn new(device: &'static dyn CharDevice) -> Self {
        Self { device }
    }
}
