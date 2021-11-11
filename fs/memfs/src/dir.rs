use crate::{BlockAllocator, Bvec, FileInode};
use alloc::boxed::Box;
use syscall::error::Errno;
use vfs::{IoctlCmd, OpenFlags, Stat, Vnode, VnodeImpl, VnodeKind, VnodeRef};

pub struct DirInode<A: BlockAllocator + Copy + 'static> {
    alloc: A,
}

impl<A: BlockAllocator + Copy + 'static> VnodeImpl for DirInode<A> {
    fn create(
        &mut self,
        _parent: VnodeRef,
        name: &str,
        kind: VnodeKind,
    ) -> Result<VnodeRef, Errno> {
        let vnode = Vnode::new(name, kind, Vnode::SEEKABLE);
        match kind {
            VnodeKind::Directory => vnode.set_data(Box::new(DirInode { alloc: self.alloc })),
            VnodeKind::Regular => vnode.set_data(Box::new(FileInode::new(Bvec::new(self.alloc)))),
            _ => todo!(),
        }
        Ok(vnode)
    }

    fn lookup(&mut self, _parent: VnodeRef, _name: &str) -> Result<VnodeRef, Errno> {
        Err(Errno::DoesNotExist)
    }

    fn remove(&mut self, _parent: VnodeRef, _name: &str) -> Result<(), Errno> {
        Ok(())
    }

    fn open(&mut self, _node: VnodeRef, _flags: OpenFlags) -> Result<usize, Errno> {
        todo!()
    }

    fn close(&mut self, _node: VnodeRef) -> Result<(), Errno> {
        todo!()
    }

    fn read(&mut self, _node: VnodeRef, _pos: usize, _data: &mut [u8]) -> Result<usize, Errno> {
        todo!()
    }

    fn write(&mut self, _node: VnodeRef, _pos: usize, _data: &[u8]) -> Result<usize, Errno> {
        todo!()
    }

    fn truncate(&mut self, _node: VnodeRef, _size: usize) -> Result<(), Errno> {
        todo!()
    }

    fn size(&mut self, _node: VnodeRef) -> Result<usize, Errno> {
        todo!()
    }

    fn stat(&mut self, _node: VnodeRef, _stat: &mut Stat) -> Result<(), Errno> {
        todo!();
    }

    fn ioctl(
        &mut self,
        node: VnodeRef,
        cmd: IoctlCmd,
        ptr: usize,
        len: usize,
    ) -> Result<usize, Errno> {
        todo!()
    }
}

impl<A: BlockAllocator + Copy + 'static> DirInode<A> {
    pub const fn new(alloc: A) -> Self {
        Self { alloc }
    }
}
