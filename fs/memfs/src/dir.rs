use vfs::{VnodeImpl, VnodeKind, VnodeRef, Vnode};
use alloc::boxed::Box;
use error::Errno;

pub struct DirInode;

impl VnodeImpl for DirInode {
    fn create(
        &mut self,
        _parent: VnodeRef,
        name: &str,
        kind: VnodeKind,
    ) -> Result<VnodeRef, Errno> {
        let vnode = Vnode::new(name, kind, Vnode::SEEKABLE);
        match kind {
            VnodeKind::Directory => vnode.set_data(Box::new(DirInode {})),
            _ => todo!()
        }
        Ok(vnode)
    }

    fn lookup(&mut self, _parent: VnodeRef, _name: &str) -> Result<VnodeRef, Errno> {
        Err(Errno::DoesNotExist)
    }

    fn remove(&mut self, _parent: VnodeRef, _name: &str) -> Result<(), Errno> {
        Ok(())
    }

    fn open(&mut self, _node: VnodeRef) -> Result<usize, Errno> {
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
}

