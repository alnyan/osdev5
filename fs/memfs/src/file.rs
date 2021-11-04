use vfs::{VnodeImpl, VnodeKind, VnodeRef, Stat, OpenFlags};
use error::Errno;
use crate::{BlockAllocator, Bvec};

pub struct FileInode<'a, A: BlockAllocator + Copy + 'static> {
    data: Bvec<'a, A>,
}

impl<'a, A: BlockAllocator + Copy + 'static> VnodeImpl for FileInode<'a, A> {
    fn create(
        &mut self,
        _parent: VnodeRef,
        _name: &str,
        _kind: VnodeKind,
    ) -> Result<VnodeRef, Errno> {
        panic!()
    }

    fn lookup(&mut self, _parent: VnodeRef, _name: &str) -> Result<VnodeRef, Errno> {
        panic!()
    }

    fn remove(&mut self, _parent: VnodeRef, _name: &str) -> Result<(), Errno> {
        panic!()
    }

    fn open(&mut self, _node: VnodeRef, _mode: OpenFlags) -> Result<usize, Errno> {
        Ok(0)
    }

    fn close(&mut self, _node: VnodeRef) -> Result<(), Errno> {
        Ok(())
    }

    fn read(&mut self, _node: VnodeRef, pos: usize, data: &mut [u8]) -> Result<usize, Errno> {
        self.data.read(pos, data)
    }

    fn write(&mut self, _node: VnodeRef, pos: usize, data: &[u8]) -> Result<usize, Errno> {
        self.data.write(pos, data)
    }

    fn truncate(&mut self, _node: VnodeRef, size: usize) -> Result<(), Errno> {
        self.data.resize((size + 4095) / 4096)
    }

    fn size(&mut self, _node: VnodeRef) -> Result<usize, Errno> {
        Ok(self.data.size())
    }

    fn stat(&mut self, _node: VnodeRef, stat: &mut Stat) -> Result<(), Errno> {
        stat.size = self.data.size() as u64;
        stat.blksize = 4096;
        stat.mode = 0o755;
        Ok(())
    }
}

impl<'a, A: BlockAllocator + Copy + 'static> FileInode<'a, A> {
    pub fn new(data: Bvec<'a, A>) -> Self {
        Self { data }
    }
}
