use crate::config::{ConfigKey, CONFIG};
use crate::fs::{devfs, MemfsBlockAlloc};
use crate::mem;
use crate::proc::{elf, Process};
use memfs::Ramfs;
use vfs::{Filesystem, Ioctx, OpenFlags};

#[inline(never)]
pub extern "C" fn init_fn(_arg: usize) -> ! {
    let proc = Process::current();

    debugln!("Running kernel init process");

    let cfg = CONFIG.lock();
    let initrd_start = cfg.get_usize(ConfigKey::InitrdBase);
    let initrd_size = cfg.get_usize(ConfigKey::InitrdSize);
    let console = cfg.get_str(ConfigKey::Console);

    if initrd_start == 0 {
        panic!("No initrd specified");
    }

    let initrd_start = mem::virtualize(initrd_start);
    let fs =
        unsafe { Ramfs::open(initrd_start as *mut u8, initrd_size, MemfsBlockAlloc {}).unwrap() };
    let root = fs.root().unwrap();

    let ioctx = Ioctx::new(root);

    let node = ioctx.find(None, "/init", true).unwrap();
    let mut file = node.open(OpenFlags::O_RDONLY).unwrap();

    proc.set_ioctx(ioctx);

    // Open stdout
    {
        let devfs_root = devfs::root();
        let tty_node = if console.is_empty() {
            devfs_root.lookup("ttyS0")
        } else {
            devfs_root.lookup(console)
        }.expect("Failed to open stdout for init process");

        let mut io = proc.io.lock();
        // TODO fd cloning?
        io.place_file(tty_node.open(OpenFlags::O_RDWR).unwrap()).unwrap();
    }

    drop(cfg);

    Process::execve(|space| elf::load_elf(space, &mut file), 0).unwrap();
    panic!("Unreachable");
}
