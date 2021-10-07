//! Timer interface

use crate::dev::Device;
use core::time::Duration;
use error::Errno;

/// Interface for generic timestamp source
pub trait TimestampSource: Device {
    /// Reads current timestamp as a [Duration] from system start time
    fn timestamp(&self) -> Result<Duration, Errno>;
}
