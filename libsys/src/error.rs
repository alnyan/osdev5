#[derive(PartialEq, Debug, Clone, Copy)]
#[repr(u32)]
pub enum Errno {
    AlreadyExists,
    BadExecutable,
    Busy,
    DeviceError,
    DoesNotExist,
    EndOfFile,
    Interrupt,
    InvalidArgument,
    InvalidFile,
    InvalidOperation,
    IsADirectory,
    NotADirectory,
    NotImplemented,
    OutOfMemory,
    PermissionDenied,
    ReadOnly,
    TimedOut,
    TooManyDescriptors,
    WouldBlock,
}

impl Errno {
    pub const fn to_negative_isize(self) -> isize {
        -(self as isize)
    }

    pub fn from_syscall(u: usize) -> Result<usize, Self> {
        let i = u as isize;
        if i < 0 {
            Err(Self::from((-i) as usize))
        } else {
            Ok(u)
        }
    }

    pub fn from_syscall_unit(u: usize) -> Result<(), Self> {
        let i = u as isize;
        if i < 0 {
            Err(Self::from((-i) as usize))
        } else {
            Ok(())
        }
    }
}

impl From<usize> for Errno {
    fn from(u: usize) -> Errno {
        unsafe { core::mem::transmute(u as u32) }
    }
}
