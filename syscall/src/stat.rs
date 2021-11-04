pub const AT_FDCWD: i32 = -2;

bitflags! {
    pub struct OpenFlags: u32 {
        const O_RDONLY =    1;
        const O_WRONLY =    2;
        const O_RDWR =      3;
        const O_ACCESS =    0xF;

        const O_CREAT =     1 << 4;
    }
}

bitflags! {
    pub struct FileMode: u32 {
        const USER_READ = 1 << 8;
        const USER_WRITE = 1 << 7;
        const USER_EXEC = 1 << 6;
        const GROUP_READ = 1 << 5;
        const GROUP_WRITE = 1 << 4;
        const GROUP_EXEC = 1 << 3;
        const OTHER_READ = 1 << 2;
        const OTHER_WRITE = 1 << 1;
        const OTHER_EXEC = 1 << 0;
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Stat {
    pub mode: u32,
    pub size: u64,
    pub blksize: u32,
}

impl FileMode {
    /// Returns default permission set for directories
    pub const fn default_dir() -> Self {
        unsafe {
            Self::from_bits_unchecked(0o755)
        }
    }

    /// Returns default permission set for regular files
    pub const fn default_reg() -> Self {
        unsafe {
            Self::from_bits_unchecked(0o644)
        }
    }
}
