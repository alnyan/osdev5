use libsys::{error::Errno, ioctl::IoctlCmd};

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
