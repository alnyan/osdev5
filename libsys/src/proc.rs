use crate::error::Errno;
use core::convert::TryFrom;
use core::fmt;

/// Wrapper type for process exit code
#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct ExitCode(i32);

/// Wrapper type for process ID
#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
#[repr(transparent)]
pub struct Pid(u32);

/// Wrapper type for thread ID
#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
#[repr(transparent)]
pub struct Tid(u32);

#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct Pgid(u32);

bitflags! {
    pub struct MemoryAccess: u32 {
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const EXEC = 1 << 2;
    }
}

bitflags! {
    pub struct MemoryMap: u32 {
        const BACKEND = 0x3;
        const ANONYMOUS = 1;

        const SHARING = 0x3 << 2;
        const PRIVATE = 1 << 2;
    }
}

impl From<i32> for ExitCode {
    fn from(f: i32) -> Self {
        Self(f)
    }
}

impl From<()> for ExitCode {
    fn from(_: ()) -> Self {
        Self(0)
    }
}

impl From<ExitCode> for i32 {
    fn from(f: ExitCode) -> Self {
        f.0
    }
}

impl Pid {
    const KERNEL_BIT: u32 = 1 << 31;
    const USER_MAX: u32 = 256;

    /// Constructs an instance of user-space PID
    pub const fn user(id: u32) -> Self {
        assert!(id < Self::USER_MAX, "PID is too high");
        if id == 0 {
            panic!("User PID cannot be zero");
        }
        Self(id)
    }

    /// Constructs an instance of kernel-space PID
    pub const fn kernel(id: u32) -> Self {
        assert!(id & Self::KERNEL_BIT == 0, "PID is too high");
        Self(id | Self::KERNEL_BIT)
    }

    /// Returns `true` if this PID belongs to a kernel process
    pub fn is_kernel(self) -> bool {
        self.0 & Self::KERNEL_BIT != 0
    }

    /// Returns address space ID of a user-space process.
    ///
    /// Panics if called on kernel process PID.
    pub fn asid(self) -> u8 {
        assert!(!self.is_kernel());
        self.0 as u8
    }

    pub fn from_option(m: Option<Self>) -> u32 {
        if let Some(pid) = m {
            u32::from(pid)
        } else {
            0
        }
    }

    pub fn to_option(m: u32) -> Option<Self> {
        if m != 0 {
            Some(Self::try_from(m).unwrap())
        } else {
            None
        }
    }
}

impl fmt::Debug for Pid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Pid(#{}{})",
            if self.is_kernel() { "K" } else { "U" },
            self.0 & !Self::KERNEL_BIT
        )
    }
}

impl TryFrom<u32> for Pid {
    type Error = Errno;

    fn try_from(raw: u32) -> Result<Pid, Errno> {
        if raw & Self::KERNEL_BIT != 0 {
            Ok(Pid::kernel(raw & !Self::KERNEL_BIT))
        } else if raw != 0 && raw < Self::USER_MAX {
            Ok(Pid::user(raw))
        } else {
            Err(Errno::InvalidArgument)
        }
    }
}

impl From<Pid> for u32 {
    fn from(pid: Pid) -> u32 {
        pid.0
    }
}

impl TryFrom<Pid> for Pgid {
    type Error = Errno;

    fn try_from(pid: Pid) -> Result<Pgid, Errno> {
        if pid.is_kernel() {
            Err(Errno::InvalidArgument)
        } else {
            Ok(Pgid(pid.0))
        }
    }
}

impl From<u32> for Pgid {
    fn from(p: u32) -> Pgid {
        Self(p)
    }
}

impl From<Pgid> for u32 {
    fn from(p: Pgid) -> u32 {
        p.0
    }
}

impl Tid {
    pub const IDLE: Tid = Tid(0);
}

impl fmt::Debug for Tid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Tid(#{})", self.0)
    }
}

impl From<u32> for Tid {
    fn from(p: u32) -> Tid {
        Self(p)
    }
}

impl From<Tid> for u32 {
    fn from(p: Tid) -> u32 {
        p.0
    }
}
