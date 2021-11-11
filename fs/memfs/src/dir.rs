use crate::{BlockAllocator, Bvec, FileInode};
use alloc::boxed::Box;
use libsys::error::Errno;
use vfs::{Vnode, VnodeImpl, VnodeKind, VnodeRef};

pub struct DirInode<A: BlockAllocator + Copy + 'static> {
    alloc: A,
}

#[auto_inode]
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
}

impl<A: BlockAllocator + Copy + 'static> DirInode<A> {
    pub const fn new(alloc: A) -> Self {
        Self { alloc }
    }
}
