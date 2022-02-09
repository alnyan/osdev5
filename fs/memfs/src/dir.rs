use crate::{BlockAllocator, Bvec, FileInode};
use alloc::boxed::Box;
use libsys::{error::Errno, stat::{Stat, DirectoryEntry, OpenFlags}, ioctl::IoctlCmd};
use vfs::{Vnode, VnodeCommon, VnodeDirectory, VnodeRef, VnodeCreateKind, VnodeData};
use core::cell::RefCell;

pub struct DirInode<A: BlockAllocator + Copy + 'static> {
    alloc: A,
}

impl<A: BlockAllocator + Copy + 'static> VnodeDirectory for DirInode<A> {
    fn create(
        &mut self,
        _parent: VnodeRef,
        name: &str,
        kind: VnodeCreateKind,
    ) -> Result<VnodeRef, Errno> {
        let data = match kind {
            VnodeCreateKind::Directory => VnodeData::Directory(RefCell::new(Some(Box::new(DirInode { alloc: self.alloc })))),
            VnodeCreateKind::File => VnodeData::File(RefCell::new(Some(Box::new(FileInode::new(Bvec::new(self.alloc)))))),
        };
        Ok(Vnode::new(name, data, Vnode::SEEKABLE | Vnode::CACHE_READDIR))
        // match kind {
        //     _ => todo!(),
        // }
        // Ok(vnode)
    }

    fn lookup(&mut self, _parent: VnodeRef, _name: &str) -> Result<VnodeRef, Errno> {
        Err(Errno::DoesNotExist)
    }

    fn remove(&mut self, _parent: VnodeRef, _name: &str) -> Result<(), Errno> {
        Ok(())
    }

    /// Read directory entries into target buffer
    fn readdir(
        &mut self,
        _node: VnodeRef,
        _pos: usize,
        _data: &mut [DirectoryEntry],
    ) -> Result<usize, Errno> {
        todo!()
    }
}

impl<A: BlockAllocator + Copy + 'static> VnodeCommon for DirInode<A> {
    fn stat(&mut self, node: VnodeRef) -> Result<Stat, Errno> {
        let props = node.props();
        Ok(Stat {
            size: 0,
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

    /// Reports the size of this filesystem object in bytes
    fn size(&mut self, _node: VnodeRef) -> Result<usize, Errno> {
        todo!()
    }

    /// Returns `true` if node is ready for an operation
    fn ready(&mut self, _node: VnodeRef, _write: bool) -> Result<bool, Errno> {
        todo!()
    }

    fn open(&mut self, _node: VnodeRef, _flags: OpenFlags) -> Result<usize, Errno> {
        Ok(0)
    }

    fn close(&mut self, _node: VnodeRef) -> Result<(), Errno> {
        Ok(())
    }
}

impl<A: BlockAllocator + Copy + 'static> DirInode<A> {
    pub const fn new(alloc: A) -> Self {
        Self { alloc }
    }
}
