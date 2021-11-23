use crate::{VnodeKind, VnodeRef, Vnode};
use alloc::rc::Rc;
use core::cell::RefCell;
use core::cmp::min;
use libsys::{
    error::Errno,
    stat::DirectoryEntry,
    traits::{Read, Seek, SeekDir, Write},
};

struct NormalFile {
    vnode: VnodeRef,
    pos: usize,
}

enum FileInner {
    Normal(NormalFile),
    // TODO
    #[allow(dead_code)]
    Socket,
}

/// Convenience wrapper type for a [File] struct reference
pub type FileRef = Rc<RefCell<File>>;

/// Structure representing a file/socket opened for access
pub struct File {
    inner: FileInner,
    flags: u32,
}

impl Read for File {
    fn read(&mut self, data: &mut [u8]) -> Result<usize, Errno> {
        if self.flags & Self::READ == 0 {
            return Err(Errno::InvalidOperation);
        }

        match &mut self.inner {
            FileInner::Normal(inner) => {
                let count = inner.vnode.read(inner.pos, data)?;
                if inner.vnode.kind() != VnodeKind::Char {
                    inner.pos += count;
                }
                Ok(count)
            }
            _ => unimplemented!(),
        }
    }
}

impl Write for File {
    fn write(&mut self, data: &[u8]) -> Result<usize, Errno> {
        if self.flags & Self::WRITE == 0 {
            return Err(Errno::ReadOnly);
        }

        match &mut self.inner {
            FileInner::Normal(inner) => {
                let count = inner.vnode.write(inner.pos, data)?;
                if inner.vnode.kind() != VnodeKind::Char {
                    inner.pos += count;
                }
                Ok(count)
            }
            _ => unimplemented!(),
        }
    }
}

impl Seek for File {
    fn seek(&mut self, off: isize, whence: SeekDir) -> Result<usize, Errno> {
        match &mut self.inner {
            FileInner::Normal(inner) => {
                if !inner.vnode.is_seekable() {
                    return Err(Errno::InvalidOperation);
                }

                let size = inner.vnode.size()?;
                let pos = match whence {
                    SeekDir::Set => min(off as usize, size),
                    _ => todo!(),
                };

                inner.pos = pos;

                Ok(pos)
            }
            _ => unimplemented!(),
        }
    }
}

impl File {
    /// File can be read
    pub const READ: u32 = 1 << 0;
    /// File can be written
    pub const WRITE: u32 = 1 << 1;
    /// File has to be closed on execve() calls
    pub const CLOEXEC: u32 = 1 << 2;

    pub const POS_CACHE_DOT: usize = usize::MAX - 1;
    pub const POS_CACHE_DOT_DOT: usize = usize::MAX;

    /// Constructs a new file handle for a regular file
    pub fn normal(vnode: VnodeRef, pos: usize, flags: u32) -> FileRef {
        Rc::new(RefCell::new(Self {
            inner: FileInner::Normal(NormalFile { vnode, pos }),
            flags,
        }))
    }

    /// Returns [VnodeRef] associated with this file, if available
    pub fn node(&self) -> Option<VnodeRef> {
        match &self.inner {
            FileInner::Normal(inner) => Some(inner.vnode.clone()),
            _ => None,
        }
    }

    /// Returns `true` if the file has to be closed when running execve() family
    /// of system calls
    pub fn is_cloexec(&self) -> bool {
        self.flags & Self::CLOEXEC != 0
    }

    pub fn is_ready(&self, write: bool) -> Result<bool, Errno> {
        match &self.inner {
            FileInner::Normal(inner) => inner.vnode.is_ready(write),
            _ => todo!(),
        }
    }

    fn cache_readdir(inner: &mut NormalFile, entries: &mut [DirectoryEntry]) -> Result<usize, Errno> {
        let mut count = entries.len();
        let mut offset = 0usize;

        if inner.pos == Self::POS_CACHE_DOT {
            if count == 0 {
                return Ok(offset);
            }

            entries[offset] = DirectoryEntry::from_str(".");
            inner.pos = Self::POS_CACHE_DOT_DOT;

            offset += 1;
            count -= 1;
        }

        if inner.pos == Self::POS_CACHE_DOT_DOT {
            if count == 0 {
                return Ok(offset);
            }

            entries[offset] = DirectoryEntry::from_str("..");
            inner.pos = 0;

            offset += 1;
            count -= 1;
        }

        if count == 0 {
            return Ok(offset);
        }

        let count = inner.vnode.for_each_entry(inner.pos, count, |i, e| {
            entries[offset + i] = DirectoryEntry::from_str(e.name());
        });
        inner.pos += count;
        Ok(offset + count)
    }

    pub fn readdir(&mut self, entries: &mut [DirectoryEntry]) -> Result<usize, Errno> {
        match &mut self.inner {
            FileInner::Normal(inner) => {
                assert_eq!(inner.vnode.kind(), VnodeKind::Directory);

                if inner.vnode.flags() & Vnode::CACHE_READDIR != 0 {
                    Self::cache_readdir(inner, entries)
                } else {
                    todo!();
                }
            },
            _ => todo!(),
        }
    }
}

impl Drop for File {
    fn drop(&mut self) {
        match &mut self.inner {
            FileInner::Normal(inner) => {
                inner.vnode.close().ok();
            }
            _ => unimplemented!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Vnode, VnodeImpl, VnodeKind, VnodeRef};
    use libsys::{stat::OpenFlags, ioctl::IoctlCmd, stat::Stat};
    use alloc::boxed::Box;
    use alloc::rc::Rc;

    struct DummyInode;

    #[auto_inode]
    impl VnodeImpl for DummyInode {
        fn create(
            &mut self,
            _at: VnodeRef,
            name: &str,
            kind: VnodeKind,
        ) -> Result<VnodeRef, Errno> {
            let node = Vnode::new(name, kind, 0);
            node.set_data(Box::new(DummyInode {}));
            Ok(node)
        }

        fn open(&mut self, _node: VnodeRef, _flags: OpenFlags) -> Result<usize, Errno> {
            Ok(0)
        }

        fn close(&mut self, _node: VnodeRef) -> Result<(), Errno> {
            Ok(())
        }

        fn read(&mut self, _node: VnodeRef, pos: usize, data: &mut [u8]) -> Result<usize, Errno> {
            #[cfg(test)]
            println!("read {} at {}", data.len(), pos);
            let len = 123;
            if pos >= len {
                return Ok(0);
            }
            let rem = core::cmp::min(len - pos, data.len());
            for i in 0..rem {
                data[i] = ((pos + i) & 0xFF) as u8;
            }
            Ok(rem)
        }

        fn write(&mut self, _node: VnodeRef, _pos: usize, _data: &[u8]) -> Result<usize, Errno> {
            Err(Errno::NotImplemented)
        }
    }

    #[test]
    fn test_normal_read() {
        let node = Vnode::new("", VnodeKind::Regular, 0);
        node.set_data(Box::new(DummyInode {}));
        let mut file = node.open(OpenFlags::O_RDONLY).unwrap();
        let mut buf = [0u8; 4096];

        assert_eq!(file.borrow_mut().read(&mut buf[0..32]).unwrap(), 32);
        for i in 0..32 {
            assert_eq!((i & 0xFF) as u8, buf[i]);
        }
        assert_eq!(file.borrow_mut().read(&mut buf[0..64]).unwrap(), 64);
        for i in 0..64 {
            assert_eq!(((i + 32) & 0xFF) as u8, buf[i]);
        }
        assert_eq!(file.borrow_mut().read(&mut buf[0..64]).unwrap(), 27);
        for i in 0..27 {
            assert_eq!(((i + 96) & 0xFF) as u8, buf[i]);
        }
    }
}
