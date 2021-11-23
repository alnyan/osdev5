//! Process file descriptors and I/O context
use alloc::collections::BTreeMap;
use libsys::{error::Errno, stat::FileDescriptor};
use vfs::{FileRef, Ioctx, VnodeRef, VnodeKind};

/// Process I/O context. Contains file tables, root/cwd info etc.
pub struct ProcessIo {
    ioctx: Option<Ioctx>,
    files: BTreeMap<u32, FileRef>,
    ctty: Option<VnodeRef>,
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

    pub fn set_ctty(&mut self, node: VnodeRef) {
        assert_eq!(node.kind(), VnodeKind::Char);
        self.ctty = Some(node);
    }

    pub fn ctty(&mut self) -> Option<VnodeRef> {
        self.ctty.clone()
    }

    /// Returns [File] struct referred to by file descriptor `idx`
    pub fn file(&mut self, fd: FileDescriptor) -> Result<FileRef, Errno> {
        self.files.get(&u32::from(fd)).cloned().ok_or(Errno::InvalidFile)
    }

    /// Returns [Ioctx] structure reference of this I/O context
    pub fn ioctx(&mut self) -> &mut Ioctx {
        self.ioctx.as_mut().unwrap()
    }

    /// Allocates a file descriptor and associates a [File] struct with it
    pub fn place_file(&mut self, file: FileRef) -> Result<FileDescriptor, Errno> {
        for idx in 0..64 {
            if self.files.get(&idx).is_none() {
                self.files.insert(idx, file);
                return Ok(FileDescriptor::from(idx));
            }
        }
        Err(Errno::TooManyDescriptors)
    }

    /// Performs [File] close and releases its associated file descriptor `idx`
    pub fn close_file(&mut self, idx: FileDescriptor) -> Result<(), Errno> {
        let res = self.files.remove(&u32::from(idx));
        assert!(res.is_some());
        Ok(())
    }

    /// Constructs a new I/O context
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
            ioctx: None,
            ctty: None,
        }
    }

    /// Assigns a descriptor number to an open file. If the number is not available,
    /// returns [Errno::AlreadyExists].
    pub fn set_file(&mut self, idx: FileDescriptor, file: FileRef) -> Result<(), Errno> {
        let idx = u32::from(idx);
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

    pub(super) fn handle_exit(&mut self) {
        self.files.clear();
        self.ioctx.take();
    }
}

impl Default for ProcessIo {
    fn default() -> Self {
        Self::new()
    }
}
