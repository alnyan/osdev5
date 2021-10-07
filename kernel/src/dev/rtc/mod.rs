//! Interfaces and drivers for real-time clock devices

use crate::dev::Device;

pub mod pl031;

// TODO define what RTC devices can do
//      alarms? read real time?
/// Interface for generic RTC device
pub trait RtcDevice: Device {}
