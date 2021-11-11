use crate::error::Errno;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SeekDir {
    Set,
    End,
    Current,
}

pub trait Read {
    fn read(&mut self, data: &mut [u8]) -> Result<usize, Errno>;
}

pub trait Seek {
    fn seek(&mut self, off: isize, whence: SeekDir) -> Result<usize, Errno>;
}

pub trait Write {
    fn write(&mut self, data: &[u8]) -> Result<usize, Errno>;
}

pub trait RandomRead {
    fn pread(&mut self, pos: usize, data: &mut [u8]) -> Result<usize, Errno>;
}

pub trait RandomWrite {
    fn pwrite(&mut self, pos: usize, data: &[u8]) -> Result<usize, Errno>;
}
