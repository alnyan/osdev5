use syscall::error::Errno;

/// Block device interface
pub trait BlockDevice {
    /// Reads blocks at offset `pos` into `buf`
    fn read(&self, pos: usize, buf: &mut [u8]) -> Result<(), Errno>;
    /// Writes blocks at offset `pos` from `buf`
    fn write(&self, pos: usize, buf: &[u8]) -> Result<(), Errno>;
    // TODO ioctl and stuff
}
