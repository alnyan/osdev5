use core::fmt;

pub const AT_FDCWD: i32 = -2;
pub const AT_EMPTY_PATH: u32 = 1 << 16;

pub const STDIN_FILENO: i32 = 0;
pub const STDOUT_FILENO: i32 = 1;
pub const STDERR_FILENO: i32 = 2;

bitflags! {
    pub struct OpenFlags: u32 {
        const O_RDONLY =    1;
        const O_WRONLY =    2;
        const O_RDWR =      3;
        const O_ACCESS =    0x7;

        const O_CREAT =     1 << 4;
        const O_EXEC =      1 << 5;
        const O_CLOEXEC =   1 << 6;
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

#[derive(Clone, Default)]
pub struct FdSet {
    bits: [u64; 2]
}

struct FdSetIter<'a> {
    idx: u32,
    set: &'a FdSet
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Stat {
    pub mode: u32,
    pub size: u64,
    pub blksize: u32,
}

impl FdSet {
    pub const fn empty() -> Self {
        Self { bits: [0; 2] }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.bits.fill(0);
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bits.iter().find(|&&x| x != 0).is_some()
    }

    #[inline]
    pub fn set(&mut self, fd: u32) {
        self.bits[(fd as usize) / 64] |= 1 << (fd % 64);
    }

    #[inline]
    pub fn clear(&mut self, fd: u32) {
        self.bits[(fd as usize) / 64] &= !(1 << (fd % 64));
    }

    #[inline]
    pub fn is_set(&self, fd: u32) -> bool {
        self.bits[(fd as usize) / 64] & (1 << (fd % 64)) != 0
    }

    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        FdSetIter {
            idx: 0,
            set: self
        }
    }
}

impl Iterator for FdSetIter<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        while self.idx < 128 {
            if self.set.is_set(self.idx) {
                let res = self.idx;
                self.idx += 1;
                return Some(res);
            }
            self.idx += 1;
        }
        None
    }
}

impl fmt::Debug for FdSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FdSet {{ ")?;
        let mut count = 0;
        for fd in self.iter() {
            if count != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", fd)?;
            count += 1;
        }
        write!(f, " }}")
    }
}

impl FileMode {
    /// Returns default permission set for directories
    pub const fn default_dir() -> Self {
        unsafe { Self::from_bits_unchecked(0o755) }
    }

    /// Returns default permission set for regular files
    pub const fn default_reg() -> Self {
        unsafe { Self::from_bits_unchecked(0o644) }
    }
}
