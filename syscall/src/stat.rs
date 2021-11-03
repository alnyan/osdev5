pub const AT_FDCWD: i32 = -2;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Stat {
    pub mode: u32,
    pub size: u64,
    pub blksize: u32,
}
