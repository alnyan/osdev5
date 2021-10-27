use error::Errno;

pub trait BlockDevice {
    fn read(&self, pos: usize, buf: &mut [u8]) -> Result<(), Errno>;
    fn write(&self, pos: usize, buf: &[u8]) -> Result<(), Errno>;
    // TODO ioctl and stuff
}
