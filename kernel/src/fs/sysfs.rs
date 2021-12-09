use crate::util::InitOnce;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicUsize, Ordering};
use fs_macros::auto_inode;
use libsys::{
    error::Errno,
    stat::{FileMode, OpenFlags, Stat},
};
use vfs::{CharDevice, CharDeviceWrapper, Vnode, VnodeImpl, VnodeKind, VnodeRef};
use core::fmt::{self, Write};
use core::str::FromStr;
use crate::debug::{self, Level};

struct NodeData<
    R: Fn(&mut [u8]) -> Result<usize, Errno>,
    W: Fn(&[u8]) -> Result<usize, Errno>,
> {
    read_func: R,
    write_func: W,
}

struct BufferWriter<'a> {
    dst: &'a mut [u8],
    pos: usize
}

impl<'a> fmt::Write for BufferWriter<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            if self.pos == self.dst.len() {
                todo!();
            }
            self.dst[self.pos] = byte;
            self.pos += 1;
        }
        Ok(())
    }
}

impl<'a> BufferWriter<'a> {
    pub const fn new(dst: &'a mut [u8]) -> Self {
        Self { dst, pos: 0 }
    }

    pub const fn count(&self) -> usize {
        self.pos
    }
}

#[auto_inode]
impl<
        R: Fn(&mut [u8]) -> Result<usize, Errno>,
        W: Fn(&[u8]) -> Result<usize, Errno>,
    > VnodeImpl for NodeData<R, W>
{
    fn open(&mut self, _node: VnodeRef, _mode: OpenFlags) -> Result<usize, Errno> {
        Ok(0)
    }

    fn close(&mut self, _node: VnodeRef) -> Result<(), Errno> {
        Ok(())
    }

    fn read(&mut self, _node: VnodeRef, pos: usize, data: &mut [u8]) -> Result<usize, Errno> {
        if pos != 0 {
            // TODO handle this
            Ok(0)
        } else {
            (self.read_func)(data)
        }
    }

    fn write(&mut self, _node: VnodeRef, pos: usize, data: &[u8]) -> Result<usize, Errno> {
        if pos != 0 {
            todo!();
        }
        (self.write_func)(data)
    }
}

impl<
        R: Fn(&mut [u8]) -> Result<usize, Errno>,
        W: Fn(&[u8]) -> Result<usize, Errno>,
    > NodeData<R, W>
{
    pub const fn new(read_func: R, write_func: W) -> Self {
        Self {
            read_func,
            write_func,
        }
    }
}

static SYSFS_ROOT: InitOnce<VnodeRef> = InitOnce::new();
static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

// TODO subdirs
fn add_generic_node<R, W>(parent: Option<VnodeRef>, name: &str, mode: FileMode, read: R, write: W)
where
    R: Fn(&mut [u8]) -> Result<usize, Errno> + 'static,
    W: Fn(&[u8]) -> Result<usize, Errno> + 'static,
{
    let node = Vnode::new(name, VnodeKind::Regular, Vnode::CACHE_STAT);
    node.props_mut().mode = mode | FileMode::S_IFREG;
    node.set_data(Box::new(NodeData::new(read, write)));

    if let Some(parent) = parent {
        parent.attach(node);
    } else {
        SYSFS_ROOT.get().attach(node);
    }
}

pub fn add_read_write_node<R, W>(parent: Option<VnodeRef>, name: &str, read: R, write: W)
where
    R: Fn(&mut [u8]) -> Result<usize, Errno> + 'static,
    W: Fn(&[u8]) -> Result<usize, Errno> + 'static,
{
    add_generic_node(parent, name, FileMode::from_bits(0o600).unwrap(), read, write)
}

pub fn add_read_node<R>(parent: Option<VnodeRef>, name: &str, read: R) where R: Fn(&mut [u8]) -> Result<usize, Errno> + 'static {
    add_generic_node(parent, name, FileMode::from_bits(0o400).unwrap(), read, |_| Err(Errno::ReadOnly))
}

pub fn add_directory(parent: Option<VnodeRef>, name: &str) -> Result<VnodeRef, Errno> {
    let node = Vnode::new(name, VnodeKind::Directory, Vnode::CACHE_READDIR | Vnode::CACHE_STAT);
    node.props_mut().mode = FileMode::from_bits(0o500).unwrap() | FileMode::S_IFDIR;

    if let Some(parent) = parent {
        parent.attach(node.clone());
    } else {
        SYSFS_ROOT.get().attach(node.clone());
    }

    Ok(node)
}

pub fn root() -> &'static VnodeRef {
    SYSFS_ROOT.get()
}

pub fn init() {
    let node = Vnode::new("", VnodeKind::Directory, Vnode::CACHE_READDIR | Vnode::CACHE_STAT);
    node.props_mut().mode = FileMode::default_dir();
    SYSFS_ROOT.init(node);

    let debug_dir = add_directory(None, "debug").unwrap();

    add_read_write_node(Some(debug_dir.clone()), "level", |buf| {
        let mut writer = BufferWriter::new(buf);
        write!(&mut writer, "{}\n", debug::LEVEL as u32).map_err(|_| Errno::InvalidArgument)?;
        Ok(writer.count())
    }, |buf| {
        let s = core::str::from_utf8(buf).map_err(|_| Errno::InvalidArgument)?;
        let value = u32::from_str(s).map_err(|_| Errno::InvalidArgument).and_then(Level::try_from)?;
        todo!()
    });

    add_read_node(None, "uptime", |buf| {
        use crate::arch::machine;
        use crate::dev::timer::TimestampSource;

        let mut writer = BufferWriter::new(buf);
        let time = machine::local_timer().timestamp()?;
        write!(&mut writer, "{} {}\n", time.as_secs(), time.subsec_nanos()).map_err(|_| Errno::InvalidArgument)?;
        Ok(writer.count())
    });
}
