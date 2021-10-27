#![no_std]

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Errno {
    InvalidArgument,
    DoesNotExist,
    NotADirectory,
    IsADirectory,
    OutOfMemory,
    WouldBlock,
    AlreadyExists,
    NotImplemented,
    TimedOut,
}
