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

#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct Pgid(u32);

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
    /// Kernel idle process always has PID of zero
    pub const IDLE: Self = Self(Self::KERNEL_BIT);

    const KERNEL_BIT: u32 = 1 << 31;

    /// Constructs an instance of user-space PID
    pub const fn user(id: u32) -> Self {
        assert!(id < 256, "PID is too high");
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

    /// Returns bit value of this pid
    pub const fn value(self) -> u32 {
        self.0
    }

    /// Constructs [Pid] from raw [u32] value
    ///
    /// # Safety
    ///
    /// Unsafe: does not check `num`
    pub const unsafe fn from_raw(num: u32) -> Self {
        Self(num)
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
