use crate::io::{AsRawFd, Error, Read, Write};
use libsys::{
    calls::{sys_openat, sys_read, sys_close},
    stat::{FileDescriptor, FileMode, OpenFlags},
};

pub struct File {
    fd: FileDescriptor,
}

impl File {
    pub fn open(path: &str) -> Result<File, Error> {
        let fd = sys_openat(None, path, FileMode::default_reg(), OpenFlags::O_RDONLY)
            .map_err(Error::from)?;
        Ok(File { fd })
    }
}

impl AsRawFd for File {
    fn as_raw_fd(&self) -> FileDescriptor {
        self.fd
    }
}

impl Drop for File {
    fn drop(&mut self) {
        sys_close(self.fd).ok();
    }
}

impl Read for File {
    fn read(&mut self, bytes: &mut [u8]) -> Result<usize, Error> {
        sys_read(self.fd, bytes).map_err(Error::from)
    }
}
