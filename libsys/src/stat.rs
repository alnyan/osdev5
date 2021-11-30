// TODO split up this file
use crate::error::Errno;
use core::fmt;

const AT_FDCWD: i32 = -2;
pub const AT_EMPTY_PATH: u32 = 1 << 16;

bitflags! {
    pub struct OpenFlags: u32 {
        const O_RDONLY =    1;
        const O_WRONLY =    2;
        const O_RDWR =      3;
        const O_ACCESS =    0x7;

        const O_CREAT =     1 << 4;
        const O_EXEC =      1 << 5;
        const O_CLOEXEC =   1 << 6;
        const O_DIRECTORY = 1 << 7;
        const O_CTTY =      1 << 8;
    }
}

bitflags! {
    pub struct FileMode: u32 {
        const FILE_TYPE = 0xF << 12;
        const S_IFREG = 0x8 << 12;
        const S_IFDIR = 0x4 << 12;
        const S_IFCHR = 0x2 << 12;

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

bitflags! {
    pub struct AccessMode: u32 {
        const R_OK = 1 << 0;
        const W_OK = 1 << 1;
        const X_OK = 1 << 2;
        const F_OK = 1 << 3;
    }
}

#[derive(Clone, Debug)]
pub struct MountOptions<'a> {
    pub device: Option<&'a str>,
    pub fs: Option<&'a str>,
    // TODO flags etc.
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct UserId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct GroupId(u32);

impl UserId {
    pub const fn root() -> Self {
        Self(0)
    }

    pub const fn is_root(self) -> bool {
        self.0 == 0
    }
}

impl From<u32> for UserId {
    #[inline(always)]
    fn from(v: u32) -> Self {
        Self(v)
    }
}

impl From<UserId> for u32 {
    #[inline(always)]
    fn from(v: UserId) -> u32 {
        v.0
    }
}

impl GroupId {
    pub const fn root() -> Self {
        Self(0)
    }

    pub const fn is_root(self) -> bool {
        self.0 == 0
    }
}

impl From<u32> for GroupId {
    #[inline(always)]
    fn from(v: u32) -> Self {
        Self(v)
    }
}

impl From<GroupId> for u32 {
    #[inline(always)]
    fn from(v: GroupId) -> u32 {
        v.0
    }
}

#[derive(Clone, Default)]
pub struct FdSet {
    bits: [u64; 2],
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct FileDescriptor(u32);

#[derive(Clone, Copy)]
pub struct DirectoryEntry {
    name: [u8; 64],
}

struct FdSetIter<'a> {
    idx: u32,
    set: &'a FdSet,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct Stat {
    pub mode: FileMode,
    pub size: u64,
    pub blksize: u32,
}

impl DirectoryEntry {
    pub const fn empty() -> Self {
        Self { name: [0; 64] }
    }

    pub fn from_str(i: &str) -> DirectoryEntry {
        let mut res = DirectoryEntry { name: [0; 64] };
        let bytes = i.as_bytes();
        res.name[..bytes.len()].copy_from_slice(bytes);
        res
    }

    pub fn as_str(&self) -> &str {
        let zero = self.name.iter().position(|&c| c == 0).unwrap();
        core::str::from_utf8(&self.name[..zero]).unwrap()
    }
}

impl fmt::Debug for DirectoryEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("DirectoryEntry")
            .field("name", &self.as_str())
            .finish()
    }
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
        self.bits.iter().any(|&x| x != 0)
    }

    #[inline]
    pub fn set(&mut self, fd: FileDescriptor) {
        self.bits[(fd.0 as usize) / 64] |= 1 << (fd.0 % 64);
    }

    #[inline]
    pub fn clear(&mut self, fd: FileDescriptor) {
        self.bits[(fd.0 as usize) / 64] &= !(1 << (fd.0 % 64));
    }

    #[inline]
    pub fn is_set(&self, fd: FileDescriptor) -> bool {
        self.bits[(fd.0 as usize) / 64] & (1 << (fd.0 % 64)) != 0
    }

    pub fn iter(&self) -> impl Iterator<Item = FileDescriptor> + '_ {
        FdSetIter { idx: 0, set: self }
    }
}

impl Iterator for FdSetIter<'_> {
    type Item = FileDescriptor;

    fn next(&mut self) -> Option<FileDescriptor> {
        while self.idx < 128 {
            if self.set.is_set(FileDescriptor(self.idx)) {
                let res = self.idx;
                self.idx += 1;
                return Some(FileDescriptor::from(res));
            }
            self.idx += 1;
        }
        None
    }
}

impl fmt::Debug for FdSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FdSet {{ ")?;
        for (count, fd) in self.iter().enumerate() {
            if count != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:?}", fd)?;
        }
        write!(f, " }}")
    }
}

impl FileMode {
    /// Returns default permission set for directories
    pub fn default_dir() -> Self {
        unsafe { Self::from_bits_unchecked(0o755) | Self::S_IFDIR }
    }

    /// Returns default permission set for regular files
    pub fn default_reg() -> Self {
        unsafe { Self::from_bits_unchecked(0o644) | Self::S_IFREG }
    }
}

fn choose<T>(q: bool, a: T, b: T) -> T {
    if q { a } else { b }
}

impl Default for FileMode {
    fn default() -> Self {
        unsafe { Self::from_bits_unchecked(0) }
    }
}

impl fmt::Display for FileMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}{}{}{}{}{}{}{}{}",
            // File type
            match *self & Self::FILE_TYPE {
                Self::S_IFCHR => 'c',
                Self::S_IFDIR => 'd',
                Self::S_IFREG => '-',
                _ => '?'
            },
            // User
            choose(self.contains(Self::USER_READ), 'r', '-'),
            choose(self.contains(Self::USER_WRITE), 'w', '-'),
            choose(self.contains(Self::USER_EXEC), 'x', '-'),
            // Group
            choose(self.contains(Self::GROUP_READ), 'r', '-'),
            choose(self.contains(Self::GROUP_WRITE), 'w', '-'),
            choose(self.contains(Self::GROUP_EXEC), 'x', '-'),
            // Other
            choose(self.contains(Self::OTHER_READ), 'r', '-'),
            choose(self.contains(Self::OTHER_WRITE), 'w', '-'),
            choose(self.contains(Self::OTHER_EXEC), 'x', '-'),
        )
    }
}

impl FileDescriptor {
    pub const STDIN: Self = Self(0);
    pub const STDOUT: Self = Self(1);
    pub const STDERR: Self = Self(2);

    pub fn from_i32(u: i32) -> Result<Option<Self>, Errno> {
        if u >= 0 {
            Ok(Some(Self(u as u32)))
        } else if u == AT_FDCWD {
            Ok(None)
        } else {
            Err(Errno::InvalidArgument)
        }
    }

    pub fn into_i32(u: Option<Self>) -> i32 {
        if let Some(u) = u {
            u.0 as i32
        } else {
            AT_FDCWD
        }
    }
}

impl From<u32> for FileDescriptor {
    fn from(u: u32) -> Self {
        Self(u)
    }
}

impl From<FileDescriptor> for u32 {
    fn from(u: FileDescriptor) -> u32 {
        u.0
    }
}
