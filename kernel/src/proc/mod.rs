//! Process and thread manipulation facilities

use crate::mem;
use crate::sync::IrqSafeSpinLock;
use alloc::{boxed::Box, collections::BTreeMap};

pub mod elf;
pub mod process;
pub use process::{Pid, Process, ProcessRef, State as ProcessState};

#[allow(missing_docs)]
pub mod wait;

pub mod sched;
pub use sched::Scheduler;
pub(self) use sched::SCHED;

macro_rules! spawn {
    (fn ($dst_arg:ident : usize) $body:block, $src_arg:expr) => {{
        #[inline(never)]
        extern "C" fn __inner_func($dst_arg : usize) -> ! {
            let __res = $body;
            {
                #![allow(unreachable_code)]
                SCHED.current_process().exit(__res);
                panic!();
            }
        }

        let __proc = $crate::proc::Process::new_kernel(__inner_func, $src_arg).unwrap();
        $crate::proc::SCHED.enqueue(__proc.id());
    }};

    (fn () $body:block) => (spawn!(fn (_arg: usize) $body, 0usize))
}

/// Performs a task switch.
///
/// See [Scheduler::switch]
pub fn switch() {
    SCHED.switch(false);
}

///
pub fn process(id: Pid) -> ProcessRef {
    PROCESSES.lock().get(&id).unwrap().clone()
}

/// Global list of all processes in the system
pub(self) static PROCESSES: IrqSafeSpinLock<BTreeMap<Pid, ProcessRef>> =
    IrqSafeSpinLock::new(BTreeMap::new());

/// Sets up initial process and enters it.
///
/// See [Scheduler::enter]
///
/// # Safety
///
/// Unsafe: May only be called once.
pub unsafe fn enter(initrd: Option<(usize, usize)>) -> ! {
    SCHED.init();
    let initrd = Box::into_raw(Box::new(initrd));

    spawn!(fn (initrd_ptr: usize) {
        use memfs::Ramfs;
        use vfs::{Filesystem, Ioctx, FileMode};
        use crate::fs::{MemfsBlockAlloc, devfs};
        debugln!("Running kernel init process");

        let initrd = unsafe { *(initrd_ptr as *const Option<(usize, usize)>) };
        if let Some((start, end)) = initrd {
            let proc = Process::current();
            let size = end - start;
            let start = mem::virtualize(start);

            infoln!("Constructing initrd filesystem in memory, this may take a while...");
            let fs = unsafe {
                Ramfs::open(start as *mut u8, size, MemfsBlockAlloc {}).unwrap()
            };
            infoln!("Done constructing ramfs");
            let root = fs.root().unwrap();
            let devfs_root = devfs::root();

            let dir = root.mkdir("dev", FileMode::default_dir()).unwrap();
            dir.mount(devfs_root.clone()).unwrap();

            let ioctx = Ioctx::new(root);

            // Open a test file
            let node = ioctx.find(None, "/init", true).unwrap();
            let mut file = node.open().unwrap();

            proc.set_ioctx(ioctx);

            // Open stdout
            {
                let mut io = proc.io.lock();
                let node = io.ioctx.as_ref().unwrap().find(None, "/dev/uart0", true).unwrap();
                // TODO fd cloning?
                io.files.push(node.open().unwrap());
            }

            Process::execve(|space| elf::load_elf(space, &mut file), 0).unwrap();
        } else {
            infoln!("No initrd, exiting!");
        }
    }, initrd as usize);
    SCHED.enter();
}
