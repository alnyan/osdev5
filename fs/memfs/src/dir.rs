use vfs::{VnodeImpl, VnodeKind, VnodeRef};
use error::Errno;

pub struct DirInode;

impl VnodeImpl for DirInode {
    fn create(
        &mut self,
        _parent: VnodeRef,
        _name: &str,
        _kind: VnodeKind,
    ) -> Result<VnodeRef, Errno> {
        todo!()
    }

    fn lookup(&mut self, _parent: VnodeRef, _name: &str) -> Result<VnodeRef, Errno> {
        panic!()
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

