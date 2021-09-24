use crate::dev::Device;
use error::Errno;

pub mod pl011;

pub trait SerialDevice: Device {
    unsafe fn send(&mut self, byte: u8) -> Result<(), Errno>;
    unsafe fn recv(&mut self, blocking: bool) -> Result<u8, Errno>;
}
