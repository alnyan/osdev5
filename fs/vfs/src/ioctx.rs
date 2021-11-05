use crate::{FileMode, FileRef, OpenFlags, VnodeKind, VnodeRef};
use error::Errno;
use libcommon::{path_component_left, path_component_right};

/// I/O context structure
#[derive(Clone)]
pub struct Ioctx {
    root: VnodeRef,
    cwd: VnodeRef,
}

impl Ioctx {
    /// Creates a new I/O context with given root node
    pub fn new(root: VnodeRef) -> Self {
        Self {
            cwd: root.clone(),
            root,
        }
    }

    fn _find(&self, mut at: VnodeRef, path: &str, follow: bool) -> Result<VnodeRef, Errno> {
        let mut element;
        let mut rest = path;

        loop {
            (element, rest) = path_component_left(rest);

            if !at.is_directory() {
                return Err(Errno::NotADirectory);
            }

            match element {
                ".." => {
                    at = at.parent();
                }
                "." => {}
                _ => break,
            }
        }

        if element.is_empty() && rest.is_empty() {
            return Ok(at);
        }
        assert!(!element.is_empty());

        let mut node = at.lookup_or_load(element)?;

        while let Some(target) = node.target() {
            assert!(node.kind() == VnodeKind::Directory);
            node = target;
        }

        if rest.is_empty() {
            Ok(node)
        } else {
            self._find(node, rest, follow)
        }
    }

    /// Looks up a path in given ioctx
    pub fn find(
        &self,
        at: Option<VnodeRef>,
        mut path: &str,
        follow: bool,
    ) -> Result<VnodeRef, Errno> {
        let at = if path.starts_with('/') {
            path = path.trim_start_matches('/');
            self.root.clone()
        } else if let Some(at) = at {
            at
        } else {
            self.cwd.clone()
        };

        self._find(at, path, follow)
    }

    /// Creates a new directory
    pub fn mkdir(
        &self,
        at: Option<VnodeRef>,
        path: &str,
        mode: FileMode,
    ) -> Result<VnodeRef, Errno> {
        let (parent, name) = path_component_right(path);
        self.find(at, parent, true)?.create(
            name.trim_start_matches('/'),
            mode,
            VnodeKind::Directory,
        )
    }

    /// Opens (and possibly creates) a filesystem path for access
    pub fn open(
        &self,
        at: Option<VnodeRef>,
        path: &str,
        mode: FileMode,
        opts: OpenFlags,
    ) -> Result<FileRef, Errno> {
        let node = match self.find(at.clone(), path, true) {
            Err(Errno::DoesNotExist) => {
                let (parent, name) = path_component_right(path);
                let at = self.find(at, parent, true)?;
                at.create(name, mode, VnodeKind::Regular)
            }
            o => o,
        }?;

        node.open(opts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Vnode, VnodeImpl, VnodeKind};
    use alloc::{boxed::Box, rc::Rc};

    pub struct DummyInode;

    impl VnodeImpl for DummyInode {
        fn create(
            &mut self,
            _at: VnodeRef,
            name: &str,
            kind: VnodeKind,
        ) -> Result<VnodeRef, Errno> {
            let vnode = Vnode::new(name, kind, 0);
            vnode.set_data(Box::new(DummyInode {}));
            Ok(vnode)
        }

        fn remove(&mut self, _at: VnodeRef, _name: &str) -> Result<(), Errno> {
            todo!()
        }

        fn lookup(&mut self, _at: VnodeRef, _name: &str) -> Result<VnodeRef, Errno> {
            Err(Errno::DoesNotExist)
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

    #[test]
    fn test_find_existing_absolute() {
        let root = Vnode::new("", VnodeKind::Directory, 0);
        let d0 = Vnode::new("dir0", VnodeKind::Directory, 0);
        let d1 = Vnode::new("dir1", VnodeKind::Directory, 0);
        let d0d0 = Vnode::new("dir0", VnodeKind::Directory, 0);
        let d0f0 = Vnode::new("file0", VnodeKind::Regular, 0);
        let d1f0 = Vnode::new("file0", VnodeKind::Regular, 0);

        root.attach(d0.clone());
        root.attach(d1.clone());
        d0.attach(d0d0.clone());
        d0.attach(d0f0.clone());
        d1.attach(d1f0.clone());

        let ioctx = Ioctx::new(root.clone());

        assert!(Rc::ptr_eq(&root, &ioctx.find(None, "/", false).unwrap()));
        assert!(Rc::ptr_eq(&root, &ioctx.find(None, "/.", false).unwrap()));
        assert!(Rc::ptr_eq(&root, &ioctx.find(None, "/./.", false).unwrap()));
        assert!(Rc::ptr_eq(
            &root,
            &ioctx.find(None, "/.///.", false).unwrap()
        ));
        assert!(Rc::ptr_eq(&root, &ioctx.find(None, "/..", false).unwrap()));
        assert!(Rc::ptr_eq(&root, &ioctx.find(None, "/../", false).unwrap()));
        assert!(Rc::ptr_eq(
            &root,
            &ioctx.find(None, "/../.", false).unwrap()
        ));
        assert!(Rc::ptr_eq(
            &root,
            &ioctx.find(None, "/../..", false).unwrap()
        ));

        assert!(Rc::ptr_eq(&d0, &ioctx.find(None, "/dir0", false).unwrap()));
        assert!(Rc::ptr_eq(&d1, &ioctx.find(None, "/dir1", false).unwrap()));
        assert!(Rc::ptr_eq(
            &d0,
            &ioctx.find(None, "/dir1/../dir0", false).unwrap()
        ));
        assert!(Rc::ptr_eq(
            &d1,
            &ioctx
                .find(None, "/dir1/../dir0/./../../.././dir1", false)
                .unwrap()
        ));

        assert!(Rc::ptr_eq(
            &d0d0,
            &ioctx.find(None, "/dir0/dir0", false).unwrap()
        ));
        assert!(Rc::ptr_eq(
            &d0d0,
            &ioctx.find(None, "/dir0/dir0/.", false).unwrap()
        ));
        assert!(Rc::ptr_eq(
            &d0,
            &ioctx.find(None, "/dir0/dir0/..", false).unwrap()
        ));
        assert!(Rc::ptr_eq(
            &d0,
            &ioctx.find(None, "/dir0/dir0/../", false).unwrap()
        ));
        assert!(Rc::ptr_eq(
            &d0,
            &ioctx.find(None, "/dir0/dir0/../.", false).unwrap()
        ));

        assert!(Rc::ptr_eq(
            &d0f0,
            &ioctx.find(None, "/dir0/file0", false).unwrap()
        ));
        assert!(Rc::ptr_eq(
            &d0f0,
            &ioctx.find(None, "/dir1/../dir0/./file0", false).unwrap()
        ));
    }

    #[test]
    fn test_find_rejects_file_dots() {
        let root = Vnode::new("", VnodeKind::Directory, 0);
        let d0 = Vnode::new("dir0", VnodeKind::Directory, 0);
        let d0f0 = Vnode::new("file0", VnodeKind::Regular, 0);

        root.attach(d0.clone());
        d0.attach(d0f0.clone());

        let ioctx = Ioctx::new(root.clone());

        assert_eq!(
            ioctx.find(None, "/dir0/file0/.", false).unwrap_err(),
            Errno::NotADirectory
        );
        assert_eq!(
            ioctx.find(None, "/dir0/file0/..", false).unwrap_err(),
            Errno::NotADirectory
        );

        // TODO handle this case
        // assert_eq!(ioctx.find(None, "/dir0/file0/").unwrap_err(), Errno::NotADirectory);
    }

    #[test]
    fn test_mkdir() {
        let root = Vnode::new("", VnodeKind::Directory, 0);
        let ioctx = Ioctx::new(root.clone());

        root.set_data(Box::new(DummyInode {}));

        assert!(ioctx.mkdir(None, "/dir0", FileMode::default_dir()).is_ok());
        assert_eq!(
            ioctx
                .mkdir(None, "/dir0", FileMode::default_dir())
                .unwrap_err(),
            Errno::AlreadyExists
        );
    }

    #[test]
    fn test_find_mount() {
        let root_outer = Vnode::new("", VnodeKind::Directory, 0);
        let dir0 = Vnode::new("dir0", VnodeKind::Directory, 0);
        let root_inner = Vnode::new("", VnodeKind::Directory, 0);
        let dir1 = Vnode::new("dir1", VnodeKind::Directory, 0);

        root_outer.clone().attach(dir0.clone());
        root_inner.clone().attach(dir1.clone());

        let ioctx = Ioctx::new(root_outer.clone());

        assert_eq!(
            ioctx.find(None, "/dir0/dir1", false).unwrap_err(),
            Errno::DoesNotExist
        );

        dir0.mount(root_inner.clone()).unwrap();

        assert!(Rc::ptr_eq(
            &root_inner,
            &ioctx.find(None, "/dir0", false).unwrap()
        ));
        assert!(Rc::ptr_eq(
            &dir1,
            &ioctx.find(None, "/dir0/dir1", false).unwrap()
        ));
        assert!(Rc::ptr_eq(
            &root_inner,
            &ioctx.find(None, "/dir0/dir1/..", false).unwrap()
        ));
        assert!(Rc::ptr_eq(
            &dir0,
            &ioctx.find(None, "/dir0/dir1/../..", false).unwrap()
        ));
        assert!(Rc::ptr_eq(
            &root_outer,
            &ioctx.find(None, "/dir0/dir1/../../..", false).unwrap()
        ));
    }
}
