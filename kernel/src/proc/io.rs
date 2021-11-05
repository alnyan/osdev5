//! Process file descriptors and I/O context
use alloc::collections::BTreeMap;
use error::Errno;
use vfs::{FileRef, Ioctx};

/// Process I/O context. Contains file tables, root/cwd info etc.
pub struct ProcessIo {
    ioctx: Option<Ioctx>,
    files: BTreeMap<usize, FileRef>,
}

impl ProcessIo {
    /// Clones this I/O context
    pub fn fork(&self) -> Result<ProcessIo, Errno> {
        // TODO
        let mut dst = ProcessIo::new();
        for (&fd, entry) in self.files.iter() {
            dst.files.insert(fd, entry.clone());
        }
        dst.ioctx = self.ioctx.clone();
        Ok(dst)
    }

    /// Returns [File] struct referred to by file descriptor `idx`
    pub fn file(&mut self, idx: usize) -> Result<FileRef, Errno> {
        self.files.get(&idx).cloned().ok_or(Errno::InvalidFile)
    }

    /// Returns [Ioctx] structure reference of this I/O context
    pub fn ioctx(&mut self) -> &mut Ioctx {
        self.ioctx.as_mut().unwrap()
    }

    /// Allocates a file descriptor and associates a [File] struct with it
    pub fn place_file(&mut self, file: FileRef) -> Result<usize, Errno> {
        for idx in 0..64 {
            if self.files.get(&idx).is_none() {
                self.files.insert(idx, file);
                return Ok(idx);
            }
        }
        Err(Errno::TooManyDescriptors)
    }

    /// Performs [File] close and releases its associated file descriptor `idx`
    pub fn close_file(&mut self, idx: usize) -> Result<(), Errno> {
        let res = self.files.remove(&idx);
        assert!(res.is_some());
        Ok(())
    }

    /// Constructs a new I/O context
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
            ioctx: None,
        }
    }

    /// Assigns a descriptor number to an open file. If the number is not available,
    /// returns [Errno::AlreadyExists].
    pub fn set_file(&mut self, idx: usize, file: FileRef) -> Result<(), Errno> {
        if self.files.get(&idx).is_none() {
            self.files.insert(idx, file);
            Ok(())
        } else {
            Err(Errno::AlreadyExists)
        }
    }

    /// Changes process I/O context: root and cwd
    pub fn set_ioctx(&mut self, ioctx: Ioctx) {
        self.ioctx.replace(ioctx);
    }

    pub(super) fn handle_cloexec(&mut self) {
        self.files.retain(|_, entry| !entry.borrow().is_cloexec());
    }
}

impl Default for ProcessIo {
    fn default() -> Self {
        Self::new()
    }
}
