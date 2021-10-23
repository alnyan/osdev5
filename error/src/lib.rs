#![no_std]

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Errno {
    InvalidArgument,
    DoesNotExist,
    NotADirectory,
    OutOfMemory,
    WouldBlock,
    AlreadyExists,
    NotImplemented,
}
