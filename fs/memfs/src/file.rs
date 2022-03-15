use crate::{BlockAllocator, Bvec};
use libsys::{
    error::Errno,
    ioctl::IoctlCmd,
    stat::{OpenFlags, Stat},
};
use vfs::{VnodeCommon, VnodeFile, VnodeRef};

pub struct FileInode<'a, A: BlockAllocator + Copy + 'static> {
    data: Bvec<'a, A>,
}

// #[auto_inode]
impl<'a, A: BlockAllocator + Copy + 'static> VnodeCommon for FileInode<'a, A> {
    fn open(&mut self, _node: VnodeRef, _mode: OpenFlags) -> Result<usize, Errno> {
        Ok(0)
    }

    fn close(&mut self, _node: VnodeRef) -> Result<(), Errno> {
        Ok(())
    }
    fn size(&mut self, _node: VnodeRef) -> Result<usize, Errno> {
        Ok(self.data.size())
    }

    fn stat(&mut self, node: VnodeRef) -> Result<Stat, Errno> {
        let props = node.props();
        Ok(Stat {
            size: self.data.size() as u64,
            blksize: 4096,
            mode: props.mode,
        })
    }

    /// Performs filetype-specific request
    fn ioctl(
        &mut self,
        _node: VnodeRef,
        _cmd: IoctlCmd,
        _ptr: usize,
        _len: usize,
    ) -> Result<usize, Errno> {
        todo!()
    }

    /// Returns `true` if node is ready for an operation
    fn ready(&mut self, _node: VnodeRef, _write: bool) -> Result<bool, Errno> {
        todo!()
    }
}

impl<'a, A: BlockAllocator + Copy + 'static> VnodeFile for FileInode<'a, A> {
    fn read(&mut self, _node: VnodeRef, pos: usize, data: &mut [u8]) -> Result<usize, Errno> {
        self.data.read(pos, data)
    }

    fn write(&mut self, _node: VnodeRef, pos: usize, data: &[u8]) -> Result<usize, Errno> {
        self.data.write(pos, data)
    }

    fn truncate(&mut self, _node: VnodeRef, size: usize) -> Result<(), Errno> {
        self.data.resize((size + 4095) / 4096)
    }
}

impl<'a, A: BlockAllocator + Copy + 'static> FileInode<'a, A> {
    pub fn new(data: Bvec<'a, A>) -> Self {
        Self { data }
    }
}
