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
    if let Some((left, right)) = path.rsplit_once('/') {
        (left.trim_end_matches('/'), right)
    } else {
        (path, "")
    }
}

#[cfg(test)]
mod tests {}
