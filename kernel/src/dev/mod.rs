use error::Errno;

pub mod serial;

pub trait Device {
    fn name() -> &'static str;

    unsafe fn enable(&mut self) -> Result<(), Errno>;
}
