#![no_std]

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Errno {
    AlreadyExists,
    BadExecutable,
    Busy,
    DeviceError,
    DoesNotExist,
    Interrupt,
    InvalidArgument,
    InvalidFile,
    InvalidOperation,
    IsADirectory,
    NotADirectory,
    NotImplemented,
    OutOfMemory,
    ReadOnly,
    TimedOut,
    TooManyDescriptors,
    WouldBlock,
}
