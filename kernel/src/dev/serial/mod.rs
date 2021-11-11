//! Module for serial device drivers

use crate::dev::Device;
use libsys::error::Errno;

#[cfg(feature = "pl011")]
pub mod pl011;

/// Generic interface for serial devices
pub trait SerialDevice: Device {
    /// Transmits (blocking) a byte through the serial device
    fn send(&self, byte: u8) -> Result<(), Errno>;
    /// Receives a byte through the serial interface.
    ///
    /// If `blocking` is `false` and there's no data in device's queue,
    /// will return [Errno::WouldBlock].
    fn recv(&self, blocking: bool) -> Result<u8, Errno>;
}
