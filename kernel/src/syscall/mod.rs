//! System call implementation

use crate::proc::{self, elf, wait, Process, ProcessIo, Thread};
use core::ops::DerefMut;
use libsys::{
    abi::SystemCall,
    debug::TraceLevel,
    error::Errno,
    ioctl::IoctlCmd,
    proc::{ExitCode, MemoryAccess, Pid, Tid},
    signal::{Signal, SignalDestination},
    stat::{
        AccessMode, DirectoryEntry, FdSet, FileDescriptor, FileMode, GroupId, MountOptions,
        OpenFlags, Stat, UserId, AT_EMPTY_PATH,
    },
    traits::{Read, Write},
};
use vfs::VnodeRef;

fn find_at_node<T: DerefMut<Target = ProcessIo>>(
    io: &mut T,
    at_fd: Option<FileDescriptor>,
    filename: &str,
    empty_path: bool,
) -> Result<VnodeRef, Errno> {
    let at = if let Some(at_fd) = at_fd {
        io.file(at_fd)?.borrow().node()
    } else {
        None
    };

    if empty_path && filename.is_empty() {
        at.ok_or(Errno::InvalidArgument)
    } else {
        io.ioctx().find(at, filename, true)
    }
}

fn _syscall(num: SystemCall, args: &[usize]) -> Result<usize, Errno> {
    todo!()
}

/// Main system call dispatcher function
pub fn syscall(num: SystemCall, args: &[usize]) -> Result<usize, Errno> {
    todo!()
}
