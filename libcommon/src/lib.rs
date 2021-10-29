#![no_std]

use error::Errno;

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

pub fn path_component_left(path: &str) -> (&str, &str) {
    if let Some((left, right)) = path.split_once('/') {
        (left, right.trim_start_matches('/'))
    } else {
        (path, "")
    }
}

pub fn path_component_right(path: &str) -> (&str, &str) {
    if let Some((left, right)) = path.trim_end_matches('/').rsplit_once('/') {
        (left.trim_end_matches('/'), right)
    } else {
        ("", path)
    }
}

pub fn read_le32(src: &[u8]) -> u32 {
    (src[0] as u32) | ((src[1] as u32) << 8) | ((src[2] as u32) << 16) | ((src[3] as u32) << 24)
}

pub fn read_le16(src: &[u8]) -> u16 {
    (src[0] as u16) | ((src[1] as u16) << 8)
}

#[cfg(test)]
mod tests {}
