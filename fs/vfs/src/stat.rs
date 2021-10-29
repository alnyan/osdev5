use core::fmt;

/// Wrapper type for file mode/permissions
#[derive(Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct FileMode(u16);

impl FileMode {
    /// File is user-readable
    pub const USER_READ: u16 = 1 << 8;
    /// File is user-writeable
    pub const USER_WRITE: u16 = 1 << 7;
    /// File is user-executable
    pub const USER_EXEC: u16 = 1 << 6;
    /// File is group-readable
    pub const GROUP_READ: u16 = 1 << 5;
    /// File is group-writeable
    pub const GROUP_WRITE: u16 = 1 << 4;
    /// File is group-executable
    pub const GROUP_EXEC: u16 = 1 << 3;
    /// File is readable by anyone
    pub const OTHER_READ: u16 = 1 << 2;
    /// File is writable by anyone
    pub const OTHER_WRITE: u16 = 1 << 1;
    /// File is executable by anyone
    pub const OTHER_EXEC: u16 = 1 << 0;

    /// Returns an empty permission set
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Returns default permission set for directories
    pub const fn default_dir() -> Self {
        Self(0o755)
    }

    /// Returns default permission set for regular files
    pub const fn default_reg() -> Self {
        Self(0o644)
    }
}

impl fmt::Debug for FileMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FileMode (")?;

        if self.0 & Self::USER_READ != 0 {
            write!(f, "r")?;
        } else {
            write!(f, "-")?;
        }
        if self.0 & Self::USER_WRITE != 0 {
            write!(f, "w")?;
        } else {
            write!(f, "-")?;
        }
        if self.0 & Self::USER_EXEC != 0 {
            write!(f, "x")?;
        } else {
            write!(f, "-")?;
        }

        if self.0 & Self::GROUP_READ != 0 {
            write!(f, "r")?;
        } else {
            write!(f, "-")?;
        }
        if self.0 & Self::GROUP_WRITE != 0 {
            write!(f, "w")?;
        } else {
            write!(f, "-")?;
        }
        if self.0 & Self::GROUP_EXEC != 0 {
            write!(f, "x")?;
        } else {
            write!(f, "-")?;
        }

        if self.0 & Self::OTHER_READ != 0 {
            write!(f, "r")?;
        } else {
            write!(f, "-")?;
        }
        if self.0 & Self::OTHER_WRITE != 0 {
            write!(f, "w")?;
        } else {
            write!(f, "-")?;
        }
        if self.0 & Self::OTHER_EXEC != 0 {
            write!(f, "x")?;
        } else {
            write!(f, "-")?;
        }

        write!(f, ")")?;
        Ok(())
    }
}
