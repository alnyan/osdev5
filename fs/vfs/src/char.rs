use crate::{VnodeData, VnodeRef};
use libsys::{error::Errno, ioctl::IoctlCmd, stat::OpenFlags};

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

    /// Performs a TTY control request
    fn ioctl(&self, cmd: IoctlCmd, ptr: usize, lim: usize) -> Result<usize, Errno>;

    /// Returns `true` if the device is ready for an operation
    fn is_ready(&self, write: bool) -> Result<bool, Errno>;
}
//
// /// Wrapper struct to attach [VnodeImpl] implementation
// /// to [CharDevice]s
// pub struct CharDeviceWrapper {
//     device: &'static dyn CharDevice,
// }
//
// // #[auto_inode(error)]
// impl VnodeCommon for CharDeviceWrapper {
//     fn open(&mut self, _node: VnodeRef, _opts: OpenFlags) -> Result<usize, Errno> {
//         Ok(0)
//     }
//
//     fn close(&mut self, _node: VnodeRef) -> Result<(), Errno> {
//         Ok(())
//     }
//
//     fn read(&mut self, _node: VnodeRef, _pos: usize, data: &mut [u8]) -> Result<usize, Errno> {
//         self.device.read(true, data)
//     }
//
//     fn write(&mut self, _node: VnodeRef, _pos: usize, data: &[u8]) -> Result<usize, Errno> {
//         self.device.write(true, data)
//     }
//
//     fn is_ready(&mut self, _node: VnodeRef, write: bool) -> Result<bool, Errno> {
//         self.device.is_ready(write)
//     }
//
//     fn ioctl(
//         &mut self,
//         _node: VnodeRef,
//         cmd: IoctlCmd,
//         ptr: usize,
//         len: usize,
//     ) -> Result<usize, Errno> {
//         self.device.ioctl(cmd, ptr, len)
//     }
// }
//
// impl CharDeviceWrapper {
//     /// Creates a wrapper for static [CharDevice] trait object to
//     /// auto-implement [VnodeImpl] trait for the device
//     pub const fn new(device: &'static dyn CharDevice) -> Self {
//         Self { device }
//     }
// }
