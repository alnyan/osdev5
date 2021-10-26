use crate::VnodeRef;
use core::cmp::min;
use error::Errno;
use libcommon::{Read, Seek, SeekDir, Write};

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

pub struct File {
    inner: FileInner,
}

impl Read for File {
    fn read(&mut self, data: &mut [u8]) -> Result<usize, Errno> {
        match &mut self.inner {
            FileInner::Normal(inner) => {
                let count = inner.vnode.read(inner.pos, data)?;
                inner.pos += count;
                Ok(count)
            }
            _ => unimplemented!(),
        }
    }
}

impl Write for File {
    fn write(&mut self, data: &[u8]) -> Result<usize, Errno> {
        match &mut self.inner {
            FileInner::Normal(inner) => {
                let count = inner.vnode.write(inner.pos, data)?;
                inner.pos += count;
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
                    return Err(Errno::InvalidArgument);
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
    pub fn normal(vnode: VnodeRef, pos: usize) -> Self {
        Self {
            inner: FileInner::Normal(NormalFile { vnode, pos }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{node::VnodeData, Filesystem, Vnode, VnodeImpl, VnodeKind, VnodeRef};
    use alloc::boxed::Box;
    use alloc::rc::Rc;
    use core::ffi::c_void;

    pub struct DummyInode;
    pub struct DummyFs;

    impl VnodeImpl for DummyInode {
        fn create(&mut self, _at: VnodeRef, _node: VnodeRef) -> Result<(), Errno> {
            Err(Errno::NotImplemented)
        }

        fn remove(&mut self, _at: VnodeRef, _name: &str) -> Result<(), Errno> {
            Err(Errno::NotImplemented)
        }

        fn open(&mut self, _node: VnodeRef) -> Result<usize, Errno> {
            Ok(0)
        }

        fn close(&mut self, _node: VnodeRef) -> Result<(), Errno> {
            Err(Errno::NotImplemented)
        }

        fn read(&mut self, _node: VnodeRef, pos: usize, data: &mut [u8]) -> Result<usize, Errno> {
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

        fn truncate(&mut self, _node: VnodeRef, _size: usize) -> Result<(), Errno> {
            Err(Errno::NotImplemented)
        }

        fn size(&mut self, _node: VnodeRef) -> Result<usize, Errno> {
            Err(Errno::NotImplemented)
        }

        fn ioctl(
            &mut self,
            _node: VnodeRef,
            _cmd: u64,
            _value: *mut c_void,
        ) -> Result<isize, Errno> {
            Err(Errno::NotImplemented)
        }
    }

    impl Filesystem for DummyFs {
        fn root(self: Rc<Self>) -> Result<VnodeRef, Errno> {
            todo!()
        }

        fn create_node(self: Rc<Self>, name: &str, kind: VnodeKind) -> Result<VnodeRef, Errno> {
            let node = Vnode::new(name, kind, 0);
            node.set_data(VnodeData {
                node: Box::new(DummyInode {}),
                fs: self,
            });
            Ok(node)
        }
    }

    #[test]
    fn test_normal_read() {
        let fs = Rc::new(DummyFs {});
        let node = fs.create_node("", VnodeKind::Regular).unwrap();
        let mut file = node.open().unwrap();

        match &file.inner {
            FileInner::Normal(inner) => {
                assert!(Rc::ptr_eq(&inner.vnode, &node));
                assert_eq!(inner.pos, 0);
            }
            _ => panic!("Invalid file.inner"),
        }

        let mut buf = [0u8; 4096];

        assert_eq!(file.read(&mut buf[0..32]).unwrap(), 32);
        for i in 0..32 {
            assert_eq!((i & 0xFF) as u8, buf[i]);
        }
        assert_eq!(file.read(&mut buf[0..64]).unwrap(), 64);
        for i in 0..64 {
            assert_eq!(((i + 32) & 0xFF) as u8, buf[i]);
        }
        assert_eq!(file.read(&mut buf[0..64]).unwrap(), 27);
        for i in 0..27 {
            assert_eq!(((i + 96) & 0xFF) as u8, buf[i]);
        }
    }
}
