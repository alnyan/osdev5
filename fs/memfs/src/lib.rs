#![feature(const_fn_trait_bound, const_mut_refs, maybe_uninit_uninit_array)]
#![no_std]

extern crate alloc;
#[cfg(test)]
#[macro_use]
extern crate std;

use alloc::{boxed::Box, rc::Rc};
use core::any::Any;
use core::cell::{Ref, RefCell};
use libsys::{
    error::Errno,
    path::{path_component_left, path_component_right},
    stat::FileMode,
};
use vfs::{BlockDevice, Filesystem, Vnode, VnodeCreateKind, VnodeData, VnodeRef};

mod block;
pub use block::{BlockAllocator, BlockRef};
mod bvec;
use bvec::Bvec;
mod tar;
use tar::{Tar, TarIterator};
mod file;
use file::FileInode;
mod dir;
use dir::DirInode;

pub struct Ramfs<A: BlockAllocator + Copy + 'static> {
    root: RefCell<Option<VnodeRef>>,
    alloc: A,
}

impl<A: BlockAllocator + Copy + 'static> Filesystem for Ramfs<A> {
    fn root(self: Rc<Self>) -> Result<VnodeRef, Errno> {
        self.root.borrow().clone().ok_or(Errno::DoesNotExist)
    }

    fn data(&self) -> Option<Ref<dyn Any>> {
        None
    }

    fn dev(self: Rc<Self>) -> Option<&'static dyn BlockDevice> {
        None
    }
}

impl<A: BlockAllocator + Copy + 'static> Ramfs<A> {
    /// # Safety
    ///
    /// Unsafe: accepts arbitrary `base` and `size` parameters
    pub unsafe fn open(base: *const u8, size: usize, alloc: A) -> Result<Rc<Self>, Errno> {
        let res = Rc::new(Self {
            root: RefCell::new(None),
            alloc,
        });
        *res.root.borrow_mut() = Some(res.clone().load_tar(base, size)?);
        Ok(res)
    }

    fn create_node_initial(self: Rc<Self>, name: &str, tar: &Tar) -> VnodeRef {
        let kind = tar.node_create_kind();
        let data = match kind {
            VnodeCreateKind::Directory => {
                VnodeData::Directory(RefCell::new(Some(Box::new(DirInode::new(self.alloc)))))
            }
            VnodeCreateKind::File => VnodeData::File(RefCell::new(None)),
        };
        let node = Vnode::new(name, data, Vnode::SEEKABLE | Vnode::CACHE_READDIR);
        node.props_mut().mode = tar.mode();
        node.set_fs(self);
        node
    }

    fn make_path(
        self: Rc<Self>,
        at: VnodeRef,
        path: &str,
        do_create: bool,
    ) -> Result<VnodeRef, Errno> {
        if path.is_empty() {
            return Ok(at);
        }
        let (element, rest) = path_component_left(path);
        assert!(!element.is_empty());

        let node = at.lookup(element);
        let node = match node {
            Some(node) => node,
            None => {
                if !do_create {
                    return Err(Errno::DoesNotExist);
                }
                // TODO file modes
                at.create(element, FileMode::default_dir(), VnodeCreateKind::Directory)?
                // todo!();
            }
        };

        if rest.is_empty() {
            Ok(node)
        } else {
            self.make_path(node, rest, do_create)
        }
    }

    unsafe fn load_tar(self: Rc<Self>, base: *const u8, size: usize) -> Result<VnodeRef, Errno> {
        let root_data =
            VnodeData::Directory(RefCell::new(Some(Box::new(DirInode::new(self.alloc)))));
        let root = Vnode::new("", root_data, Vnode::SEEKABLE | Vnode::CACHE_READDIR);
        root.set_fs(self.clone());
        root.props_mut().mode = FileMode::default_dir();

        // 1. Create all the paths in TAR
        for block in TarIterator::new(base, base.add(size)) {
            let (dirname, basename) = path_component_right(block.path()?);

            let parent = self.clone().make_path(root.clone(), dirname, true)?;
            let node = self.clone().create_node_initial(basename, block);
            parent.attach(node);
        }

        // 2. Setup data blocks
        for block in TarIterator::new(base, base.add(size)) {
            if block.is_file() {
                // Will not create any dirs
                let node = self.clone().make_path(root.clone(), block.path()?, false)?;

                #[cfg(feature = "cow")]
                {
                    let data = block.data();
                    node.as_file()
                        .unwrap()
                        .replace(Some(Box::new(FileInode::new(Bvec::new_copy_on_write(
                            self.alloc,
                            data.as_ptr(),
                            data.len(),
                        )))));
                }
                #[cfg(not(feature = "cow"))]
                {
                    node.set_data(Box::new(FileInode::new(Bvec::new(self.alloc))));

                    let size = block.size();
                    node.truncate(size)?;
                    if node.write(0, block.data())? != size {
                        return Err(Errno::InvalidArgument);
                    }
                }
            }
        }

        Ok(root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use libsys::{traits::Read, stat::{UserId, GroupId, OpenFlags}};
    use vfs::Ioctx;

    #[test]
    fn ramfs_open() {
        #[derive(Clone, Copy)]
        struct A;
        unsafe impl BlockAllocator for A {
            fn alloc(&self) -> *mut u8 {
                let b = Box::leak(Box::new([0; block::SIZE]));
                b.as_mut_ptr() as *mut _
            }
            unsafe fn dealloc(&self, ptr: *mut u8) {
                drop(Box::from_raw(ptr as *mut [u8; block::SIZE]));
            }
        }
        unsafe impl Sync for A {}

        let data = include_str!("../test/test1.tar");
        let fs = unsafe { Ramfs::open(data.as_ptr(), data.bytes().len(), A {}).unwrap() };

        let root = fs.root().unwrap();
        let ioctx = Ioctx::new(root.clone(), UserId::root(), GroupId::root());

        assert!(Rc::ptr_eq(&ioctx.find(None, "/", true).unwrap(), &root));

        let node = ioctx.find(None, "/test1.txt", true).unwrap();
        let mut file = node.open(OpenFlags::O_RDONLY).unwrap();
        let mut buf = [0u8; 1024];

        assert_eq!(file.borrow_mut().read(&mut buf).unwrap(), 20);
        let s = core::str::from_utf8(&buf[..20]).unwrap();
        assert_eq!(s, "This is a test file\n");
    }
}
