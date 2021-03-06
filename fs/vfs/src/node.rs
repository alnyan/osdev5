use crate::{File, FileRef, Filesystem, Ioctx};
use alloc::{borrow::ToOwned, boxed::Box, rc::Rc, string::String, vec::Vec};
use core::cell::{Ref, RefCell, RefMut};
use core::fmt;
use libsys::{
    error::Errno,
    ioctl::IoctlCmd,
    stat::{AccessMode, DirectoryEntry, FileMode, OpenFlags, Stat},
};

/// Convenience type alias for [Rc<Vnode>]
pub type VnodeRef = Rc<Vnode>;

/// List of possible vnode types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VnodeKind {
    /// Node is a directory with create/lookup/remove operations
    Directory,
    /// Node is a regular file
    Regular,
    /// Node is a character device
    Char,
    /// Node is a block device
    Block,
}

pub(crate) struct TreeNode {
    parent: Option<VnodeRef>,
    children: Vec<VnodeRef>,
}

/// File property cache struct
pub struct VnodeProps {
    /// Node permissions and type
    pub mode: FileMode,
}

/// Virtual filesystem node struct, generalizes access to
/// underlying real filesystems
pub struct Vnode {
    name: String,
    tree: RefCell<TreeNode>,
    props: RefCell<VnodeProps>,

    kind: VnodeKind,
    flags: u32,

    target: RefCell<Option<VnodeRef>>,
    fs: RefCell<Option<Rc<dyn Filesystem>>>,
    data: RefCell<Option<Box<dyn VnodeImpl>>>,
}

/// Interface for "inode" of a real filesystem
pub trait VnodeImpl {
    // Directory-only operations
    /// Creates a new vnode, sets it up, attaches it (in real FS) to `at` with `name` and
    /// returns it
    fn create(&mut self, at: VnodeRef, name: &str, kind: VnodeKind) -> Result<VnodeRef, Errno>;
    /// Removes the filesystem inode from its parent by erasing its directory entry
    fn remove(&mut self, at: VnodeRef, name: &str) -> Result<(), Errno>;
    /// Looks up a corresponding directory entry for `name`. If present, loads its inode from
    /// storage medium and returns a new vnode associated with it.
    fn lookup(&mut self, at: VnodeRef, name: &str) -> Result<VnodeRef, Errno>;

    /// Opens a vnode for access. Returns initial file position.
    fn open(&mut self, node: VnodeRef, opts: OpenFlags) -> Result<usize, Errno>;
    /// Closes a vnode
    fn close(&mut self, node: VnodeRef) -> Result<(), Errno>;

    /// Changes file's underlying storage size
    fn truncate(&mut self, node: VnodeRef, size: usize) -> Result<(), Errno>;
    /// Reads `data.len()` bytes into the buffer from file offset `pos`
    fn read(&mut self, node: VnodeRef, pos: usize, data: &mut [u8]) -> Result<usize, Errno>;
    /// Writes `data.len()` bytes from the buffer to file offset `pos`.
    /// Resizes the file storage if necessary.
    fn write(&mut self, node: VnodeRef, pos: usize, data: &[u8]) -> Result<usize, Errno>;

    /// Read directory entries into target buffer
    fn readdir(
        &mut self,
        node: VnodeRef,
        pos: usize,
        data: &mut [DirectoryEntry],
    ) -> Result<usize, Errno>;

    /// Retrieves file status
    fn stat(&mut self, node: VnodeRef) -> Result<Stat, Errno>;

    /// Reports the size of this filesystem object in bytes
    fn size(&mut self, node: VnodeRef) -> Result<usize, Errno>;

    /// Returns `true` if node is ready for an operation
    fn is_ready(&mut self, node: VnodeRef, write: bool) -> Result<bool, Errno>;

    /// Performs filetype-specific request
    fn ioctl(
        &mut self,
        node: VnodeRef,
        cmd: IoctlCmd,
        ptr: usize,
        len: usize,
    ) -> Result<usize, Errno>;
}

impl Vnode {
    /// If set, allows [File] structures associated with a [Vnode] to
    /// be seeked to arbitrary offsets
    pub const SEEKABLE: u32 = 1 << 0;

    /// If set, readdir() uses only in-memory node tree
    pub const CACHE_READDIR: u32 = 1 << 1;
    /// If set, stat() uses only in-memory stat data
    pub const CACHE_STAT: u32 = 1 << 2;

    /// Constructs a new [Vnode], wrapping it in [Rc]. The resulting node
    /// then needs to have [Vnode::set_data()] called on it to be usable.
    pub fn new(name: &str, kind: VnodeKind, flags: u32) -> VnodeRef {
        Rc::new(Self {
            name: name.to_owned(),
            kind,
            flags,
            props: RefCell::new(VnodeProps {
                mode: FileMode::empty(),
            }),
            tree: RefCell::new(TreeNode {
                parent: None,
                children: Vec::new(),
            }),
            target: RefCell::new(None),
            fs: RefCell::new(None),
            data: RefCell::new(None),
        })
    }

    /// Returns [Vnode]'s path element name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns a borrowed reference to cached file properties
    pub fn props_mut(&self) -> RefMut<VnodeProps> {
        self.props.borrow_mut()
    }

    /// Returns a borrowed reference to cached file properties
    pub fn props(&self) -> Ref<VnodeProps> {
        self.props.borrow()
    }

    /// Sets an associated [VnodeImpl] for the [Vnode]
    pub fn set_data(&self, data: Box<dyn VnodeImpl>) {
        *self.data.borrow_mut() = Some(data);
    }

    /// Sets an associated [Filesystem] for the [Vnode]
    pub fn set_fs(&self, fs: Rc<dyn Filesystem>) {
        *self.fs.borrow_mut() = Some(fs);
    }

    /// Returns a reference to the associated [VnodeImpl]
    pub fn data(&self) -> RefMut<Option<Box<dyn VnodeImpl>>> {
        self.data.borrow_mut()
    }

    /// Returns the associated [Fileystem]
    pub fn fs(&self) -> Option<Rc<dyn Filesystem>> {
        self.fs.borrow().clone()
    }

    /// Returns `true` if the vnode represents a directory
    pub fn is_directory(&self) -> bool {
        self.kind == VnodeKind::Directory
    }

    /// Returns `true` if the vnode allows arbitrary seeking
    pub fn is_seekable(&self) -> bool {
        self.flags & Self::SEEKABLE != 0
    }

    /// Returns kind of the vnode
    #[inline(always)]
    pub const fn kind(&self) -> VnodeKind {
        self.kind
    }

    /// Returns flags of the vnode
    #[inline(always)]
    pub const fn flags(&self) -> u32 {
        self.flags
    }

    // Tree operations

    /// Attaches `child` vnode to `self` in in-memory tree. NOTE: does not
    /// actually perform any real filesystem operations. Used to build
    /// hierarchies for in-memory or volatile filesystems.
    pub fn attach(self: &VnodeRef, child: VnodeRef) {
        let parent_clone = self.clone();
        let mut parent_borrow = self.tree.borrow_mut();
        assert!(child
            .tree
            .borrow_mut()
            .parent
            .replace(parent_clone)
            .is_none());
        parent_borrow.children.push(child);
    }

    fn detach(self: &VnodeRef) {
        let mut self_borrow = self.tree.borrow_mut();
        let parent = self_borrow.parent.take().unwrap();
        let mut parent_borrow = parent.tree.borrow_mut();
        let index = parent_borrow
            .children
            .iter()
            .position(|it| Rc::ptr_eq(it, self))
            .unwrap();
        parent_borrow.children.remove(index);
    }

    /// Attaches some filesystem's root directory node at another directory
    pub fn mount(self: &VnodeRef, root: VnodeRef) -> Result<(), Errno> {
        if !self.is_directory() {
            return Err(Errno::NotADirectory);
        }
        if !root.is_directory() {
            return Err(Errno::NotADirectory);
        }
        if self.target.borrow().is_some() {
            return Err(Errno::Busy);
        }

        let mut child_borrow = root.tree.borrow_mut();
        if child_borrow.parent.is_some() {
            return Err(Errno::Busy);
        }
        child_borrow.parent = Some(self.clone());
        *self.target.borrow_mut() = Some(root.clone());

        Ok(())
    }

    /// Returns this vnode's parent or itself if it has none
    pub fn parent(self: &VnodeRef) -> VnodeRef {
        self.tree.borrow().parent.as_ref().unwrap_or(self).clone()
    }

    /// Returns this vnode's mount target (for directories)
    pub fn target(self: &VnodeRef) -> Option<VnodeRef> {
        self.target.borrow().clone()
    }

    /// Looks up a child `name` in in-memory tree cache
    pub fn lookup(self: &VnodeRef, name: &str) -> Option<VnodeRef> {
        assert!(self.is_directory());
        self.tree
            .borrow()
            .children
            .iter()
            .find(|e| e.name == name)
            .cloned()
    }

    pub(crate) fn for_each_entry<F: FnMut(usize, &VnodeRef)>(
        &self,
        offset: usize,
        limit: usize,
        mut f: F,
    ) -> usize {
        assert!(self.is_directory());
        let mut count = 0;
        for (index, item) in self
            .tree
            .borrow()
            .children
            .iter()
            .skip(offset)
            .take(limit)
            .enumerate()
        {
            f(index, item);
            count += 1;
        }
        count
    }

    /// Looks up a child `name` in `self`. Will first try looking up a cached
    /// vnode and will load it from disk if it's missing.
    pub fn lookup_or_load(self: &VnodeRef, name: &str) -> Result<VnodeRef, Errno> {
        if let Some(node) = self.lookup(name) {
            Ok(node)
        } else if let Some(ref mut data) = *self.data() {
            let vnode = data.lookup(self.clone(), name)?;
            if let Some(fs) = self.fs() {
                vnode.set_fs(fs);
            }
            self.attach(vnode.clone());
            Ok(vnode)
        } else {
            Err(Errno::DoesNotExist)
        }
    }

    /// Creates a new node `name` in `self`
    pub fn create(
        self: &VnodeRef,
        name: &str,
        mode: FileMode,
        kind: VnodeKind,
    ) -> Result<VnodeRef, Errno> {
        if self.kind != VnodeKind::Directory {
            return Err(Errno::NotADirectory);
        }
        if name.contains('/') {
            return Err(Errno::InvalidArgument);
        }

        match self.lookup_or_load(name) {
            Err(Errno::DoesNotExist) => {}
            Ok(_) => return Err(Errno::AlreadyExists),
            e => return e,
        };

        if let Some(ref mut data) = *self.data() {
            let vnode = data.create(self.clone(), name, kind)?;
            if let Some(fs) = self.fs() {
                vnode.set_fs(fs);
            }
            vnode.props.borrow_mut().mode = mode;
            self.attach(vnode.clone());
            Ok(vnode)
        } else {
            Err(Errno::NotImplemented)
        }
    }

    /// Removes a directory entry `name` from `self`
    pub fn unlink(self: &VnodeRef, name: &str) -> Result<(), Errno> {
        if self.kind != VnodeKind::Directory {
            return Err(Errno::NotADirectory);
        }
        if name.contains('/') {
            return Err(Errno::InvalidArgument);
        }

        if let Some(ref mut data) = *self.data() {
            let vnode = self.lookup(name).ok_or(Errno::DoesNotExist)?;
            data.remove(self.clone(), name)?;
            vnode.detach();
            Ok(())
        } else {
            Err(Errno::NotImplemented)
        }
    }

    /// Opens a vnode for access
    pub fn open(self: &VnodeRef, flags: OpenFlags) -> Result<FileRef, Errno> {
        let mut open_flags = 0;
        if flags.contains(OpenFlags::O_DIRECTORY) {
            if self.kind != VnodeKind::Directory {
                return Err(Errno::NotADirectory);
            }
            if flags & OpenFlags::O_ACCESS != OpenFlags::O_RDONLY {
                return Err(Errno::IsADirectory);
            }

            open_flags = File::READ;
        } else {
            if self.kind == VnodeKind::Directory {
                return Err(Errno::IsADirectory);
            }

            match flags & OpenFlags::O_ACCESS {
                OpenFlags::O_RDONLY => open_flags |= File::READ,
                OpenFlags::O_WRONLY => open_flags |= File::WRITE,
                OpenFlags::O_RDWR => open_flags |= File::READ | File::WRITE,
                _ => unimplemented!(),
            }
        }

        if flags.contains(OpenFlags::O_CLOEXEC) {
            open_flags |= File::CLOEXEC;
        }

        if self.kind == VnodeKind::Directory && self.flags & Vnode::CACHE_READDIR != 0 {
            Ok(File::normal(self.clone(), File::POS_CACHE_DOT, open_flags))
        } else if let Some(ref mut data) = *self.data() {
            let pos = data.open(self.clone(), flags)?;
            Ok(File::normal(self.clone(), pos, open_flags))
        } else {
            Err(Errno::NotImplemented)
        }
    }

    /// Closes a vnode
    pub fn close(self: &VnodeRef) -> Result<(), Errno> {
        if self.kind == VnodeKind::Directory && self.flags & Vnode::CACHE_READDIR != 0 {
            Ok(())
        } else if let Some(ref mut data) = *self.data() {
            data.close(self.clone())
        } else {
            Err(Errno::NotImplemented)
        }
    }

    /// Reads data from offset `pos` into `buf`
    pub fn read(self: &VnodeRef, pos: usize, buf: &mut [u8]) -> Result<usize, Errno> {
        if self.kind == VnodeKind::Directory {
            Err(Errno::IsADirectory)
        } else if let Some(ref mut data) = *self.data() {
            data.read(self.clone(), pos, buf)
        } else {
            Err(Errno::NotImplemented)
        }
    }

    /// Writes data from `buf` to offset `pos`
    pub fn write(self: &VnodeRef, pos: usize, buf: &[u8]) -> Result<usize, Errno> {
        if self.kind == VnodeKind::Directory {
            Err(Errno::IsADirectory)
        } else if let Some(ref mut data) = *self.data() {
            data.write(self.clone(), pos, buf)
        } else {
            Err(Errno::NotImplemented)
        }
    }

    /// Resizes the vnode data
    pub fn truncate(self: &VnodeRef, size: usize) -> Result<(), Errno> {
        if self.kind != VnodeKind::Regular {
            Err(Errno::IsADirectory)
        } else if let Some(ref mut data) = *self.data() {
            data.truncate(self.clone(), size)
        } else {
            Err(Errno::NotImplemented)
        }
    }

    /// Returns current vnode data size
    pub fn size(self: &VnodeRef) -> Result<usize, Errno> {
        if let Some(ref mut data) = *self.data() {
            data.size(self.clone())
        } else {
            Err(Errno::NotImplemented)
        }
    }

    /// Reports file status
    pub fn stat(self: &VnodeRef) -> Result<Stat, Errno> {
        if self.flags & Self::CACHE_STAT != 0 {
            let props = self.props();
            Ok(Stat {
                blksize: 0,
                size: 0,
                mode: props.mode,
            })
        } else if let Some(ref mut data) = *self.data() {
            data.stat(self.clone())
        } else {
            Err(Errno::NotImplemented)
        }
    }

    /// Performs node-specific requests
    pub fn ioctl(self: &VnodeRef, cmd: IoctlCmd, ptr: usize, len: usize) -> Result<usize, Errno> {
        if let Some(ref mut data) = *self.data() {
            data.ioctl(self.clone(), cmd, ptr, len)
        } else {
            Err(Errno::NotImplemented)
        }
    }

    /// Returns `true` if the node is ready for operation
    pub fn is_ready(self: &VnodeRef, write: bool) -> Result<bool, Errno> {
        if let Some(ref mut data) = *self.data() {
            data.is_ready(self.clone(), write)
        } else {
            Err(Errno::NotImplemented)
        }
    }

    /// Checks if given [Ioctx] has `access` permissions to the vnode
    pub fn check_access(&self, _ioctx: &Ioctx, access: AccessMode) -> Result<(), Errno> {
        let props = self.props.borrow();
        let mode = props.mode;

        if access.contains(AccessMode::F_OK) {
            if access.intersects(AccessMode::R_OK | AccessMode::W_OK | AccessMode::X_OK) {
                return Err(Errno::InvalidArgument);
            }
        } else {
            if access.contains(AccessMode::F_OK) {
                return Err(Errno::InvalidArgument);
            }

            // Check user
            if access.contains(AccessMode::R_OK) && !mode.contains(FileMode::USER_READ) {
                return Err(Errno::PermissionDenied);
            }
            if access.contains(AccessMode::W_OK) && !mode.contains(FileMode::USER_WRITE) {
                return Err(Errno::PermissionDenied);
            }
            if access.contains(AccessMode::X_OK) && !mode.contains(FileMode::USER_EXEC) {
                return Err(Errno::PermissionDenied);
            }

            // TODO check group
            // TODO check other
        }

        Ok(())
    }
}

impl fmt::Debug for Vnode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Vnode({:?})", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use libsys::{ioctl::IoctlCmd, stat::OpenFlags, stat::Stat};
    pub struct DummyInode;

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

        fn remove(&mut self, _at: VnodeRef, _name: &str) -> Result<(), Errno> {
            Ok(())
        }

        fn lookup(&mut self, _at: VnodeRef, _name: &str) -> Result<VnodeRef, Errno> {
            Err(Errno::DoesNotExist)
        }
    }

    #[test]
    fn test_parent() {
        let root = Vnode::new("", VnodeKind::Directory, 0);
        let node = Vnode::new("dir0", VnodeKind::Directory, 0);

        root.attach(node.clone());

        assert!(Rc::ptr_eq(&root.parent(), &root));
        assert!(Rc::ptr_eq(&node.parent(), &root));
    }

    #[test]
    fn test_mkdir_unlink() {
        let root = Vnode::new("", VnodeKind::Directory, 0);

        root.set_data(Box::new(DummyInode {}));

        let node = root
            .create("test", FileMode::default_dir(), VnodeKind::Directory)
            .unwrap();

        assert_eq!(
            root.create("test", FileMode::default_dir(), VnodeKind::Directory)
                .unwrap_err(),
            Errno::AlreadyExists
        );

        assert_eq!(node.props.borrow().mode, FileMode::default_dir());
        assert!(Rc::ptr_eq(&node, &root.lookup("test").unwrap()));
        assert!(node.data.borrow().is_some());

        root.unlink("test").unwrap();

        assert!(root.lookup("test").is_none());
    }

    #[test]
    fn test_lookup_attach_detach() {
        let root = Vnode::new("", VnodeKind::Directory, 0);
        let dir0 = Vnode::new("dir0", VnodeKind::Directory, 0);
        let dir1 = Vnode::new("dir1", VnodeKind::Directory, 0);

        root.attach(dir0.clone());
        root.attach(dir1.clone());

        assert!(Rc::ptr_eq(&dir0, &root.lookup("dir0").unwrap()));
        assert!(Rc::ptr_eq(&dir1, &root.lookup("dir1").unwrap()));
        assert!(Rc::ptr_eq(
            &root,
            dir0.tree.borrow().parent.as_ref().unwrap()
        ));
        assert!(Rc::ptr_eq(
            &root,
            dir1.tree.borrow().parent.as_ref().unwrap()
        ));
        assert!(root.lookup("dir2").is_none());

        dir0.detach();

        assert!(Rc::ptr_eq(&dir1, &root.lookup("dir1").unwrap()));
        assert!(Rc::ptr_eq(
            &root,
            dir1.tree.borrow().parent.as_ref().unwrap()
        ));
        assert!(dir0.tree.borrow().parent.is_none());
        assert!(root.lookup("dir0").is_none());
        assert!(root.lookup("dir2").is_none());
    }
}
