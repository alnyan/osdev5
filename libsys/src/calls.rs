use crate::abi::SystemCall;
use crate::{
    debug::TraceLevel,
    error::Errno,
    ioctl::IoctlCmd,
    proc::{ExitCode, MemoryAccess, MemoryMap, Pid},
    signal::{Signal, SignalDestination},
    stat::{
        AccessMode, DirectoryEntry, FdSet, FileDescriptor, FileMode, GroupId, MountOptions,
        OpenFlags, Stat, UserId,
    },
};
use core::time::Duration;

// TODO document the syscall ABI

// TODO move this to libusr
macro_rules! syscall {
    ($num:expr) => {{
        let mut res: usize;
        asm!("svc #0", out("x0") res, in("x8") $num.repr(), options(nostack));
        res
    }};
    ($num:expr, $a0:expr) => {{
        let mut res: usize = $a0;
        asm!("svc #0",
             inout("x0") res,
             in("x8") $num.repr(), options(nostack));
        res
    }};
    ($num:expr, $a0:expr, $a1:expr) => {{
        let mut res: usize = $a0;
        asm!("svc #0",
             inout("x0") res, in("x1") $a1,
             in("x8") $num.repr(), options(nostack));
        res
    }};
    ($num:expr, $a0:expr, $a1:expr, $a2:expr) => {{
        let mut res: usize = $a0;
        asm!("svc #0",
             inout("x0") res, in("x1") $a1, in("x2") $a2,
             in("x8") $num.repr(), options(nostack));
        res
    }};
    ($num:expr, $a0:expr, $a1:expr, $a2:expr, $a3:expr) => {{
        let mut res: usize = $a0;
        asm!("svc #0",
             inout("x0") res, in("x1") $a1, in("x2") $a2,
             in("x3") $a3, in("x8") $num.repr(), options(nostack));
        res
    }};
    ($num:expr, $a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr) => {{
        let mut res: usize = $a0;
        asm!("svc #0",
             inout("x0") res, in("x1") $a1, in("x2") $a2,
             in("x3") $a3, in("x4") $a4, in("x8") $num.repr(), options(nostack));
        res
    }};
}

/// Integer/size argument
macro_rules! argn {
    ($a:expr) => {
        $a as usize
    };
}
/// Pointer/base argument
macro_rules! argp {
    ($a:expr) => {
        $a as usize
    };
}
// /// Immutable pointer/base argument
// macro_rules! argpi {
//     ($a:expr) => ($a as *const core::ffi::c_void as usize)
// }

/// # Safety
///
/// System call
#[inline(always)]
pub fn sys_exit(code: ExitCode) -> ! {
    unsafe {
        syscall!(SystemCall::Exit, argn!(i32::from(code)), argn!(0));
    }
    unreachable!();
}

/// # Safety
///
/// System call
#[inline(always)]
pub fn sys_close(fd: FileDescriptor) -> Result<(), Errno> {
    Errno::from_syscall_unit(unsafe { syscall!(SystemCall::Close, argn!(u32::from(fd))) })
}

/// # Safety
///
/// System call
#[inline(always)]
pub fn sys_ex_nanosleep(ns: u64, rem: &mut [u64; 2]) -> Result<(), Errno> {
    Errno::from_syscall_unit(unsafe {
        syscall!(SystemCall::Sleep, argn!(ns), argp!(rem.as_mut_ptr()))
    })
}

/// # Safety
///
/// System call
#[inline(always)]
pub fn sys_ex_debug_trace(level: TraceLevel, msg: &[u8]) -> Result<(), Errno> {
    Errno::from_syscall_unit(unsafe {
        syscall!(
            SystemCall::DebugTrace,
            argn!(level.repr()),
            argp!(msg.as_ptr()),
            argn!(msg.len())
        )
    })
}

/// # Safety
///
/// System call
#[inline(always)]
pub fn sys_openat(
    at: Option<FileDescriptor>,
    pathname: &str,
    mode: FileMode,
    flags: OpenFlags,
) -> Result<FileDescriptor, Errno> {
    Errno::from_syscall(unsafe {
        syscall!(
            SystemCall::Open,
            argn!(FileDescriptor::into_i32(at)),
            argp!(pathname.as_ptr()),
            argn!(pathname.len()),
            argn!(mode.bits()),
            argn!(flags.bits())
        )
    })
    .map(|e| FileDescriptor::from(e as u32))
}

/// # Safety
///
/// System call
#[inline(always)]
pub fn sys_read(fd: FileDescriptor, data: &mut [u8]) -> Result<usize, Errno> {
    Errno::from_syscall(unsafe {
        syscall!(
            SystemCall::Read,
            argn!(u32::from(fd)),
            argp!(data.as_mut_ptr()),
            argn!(data.len())
        )
    })
}

#[inline(always)]
pub fn sys_write(fd: FileDescriptor, data: &[u8]) -> Result<usize, Errno> {
    Errno::from_syscall(unsafe {
        syscall!(
            SystemCall::Write,
            argn!(u32::from(fd)),
            argp!(data.as_ptr()),
            argn!(data.len())
        )
    })
}

/// # Safety
///
/// System call
#[inline(always)]
pub fn sys_fstatat(
    at: Option<FileDescriptor>,
    pathname: &str,
    statbuf: &mut Stat,
    flags: u32,
) -> Result<(), Errno> {
    Errno::from_syscall_unit(unsafe {
        syscall!(
            SystemCall::FileStatus,
            argn!(FileDescriptor::into_i32(at)),
            argp!(pathname.as_ptr()),
            argn!(pathname.len()),
            argp!(statbuf as *mut Stat),
            argn!(flags)
        )
    })
}

/// # Safety
///
/// System call
#[inline(always)]
pub unsafe fn sys_fork() -> Result<Option<Pid>, Errno> {
    Errno::from_syscall(syscall!(SystemCall::Fork)).and_then(|res| {
        if res != 0 {
            Pid::try_from(res as u32).map(Some)
        } else {
            Ok(None)
        }
    })
}

/// # Safety
///
/// System call
#[inline(always)]
pub fn sys_execve(pathname: &str, argv: &[&str]) -> Result<(), Errno> {
    Errno::from_syscall_unit(unsafe {
        syscall!(
            SystemCall::Exec,
            argp!(pathname.as_ptr()),
            argn!(pathname.len()),
            argp!(argv.as_ptr()),
            argn!(argv.len())
        )
    })
}

/// # Safety
///
/// System call
#[inline(always)]
pub fn sys_waitpid(pid: Pid, status: &mut i32) -> Result<(), Errno> {
    Errno::from_syscall_unit(unsafe {
        syscall!(
            SystemCall::WaitPid,
            argn!(u32::from(pid)),
            argp!(status as *mut i32)
        )
    })
}

/// # Safety
///
/// System call
#[inline(always)]
pub fn sys_ioctl(
    fd: FileDescriptor,
    cmd: IoctlCmd,
    ptr: usize,
    len: usize,
) -> Result<usize, Errno> {
    Errno::from_syscall(unsafe {
        syscall!(
            SystemCall::Ioctl,
            argn!(u32::from(fd)),
            argn!(cmd),
            argn!(ptr),
            argn!(len)
        )
    })
}

#[inline(always)]
pub fn sys_ex_getcputime() -> Result<Duration, Errno> {
    Errno::from_syscall(unsafe { syscall!(SystemCall::GetCpuTime) })
        .map(|e| Duration::from_nanos(e as u64))
}

#[inline(always)]
pub fn sys_ex_signal(entry: usize, stack: usize) -> Result<(), Errno> {
    Errno::from_syscall_unit(unsafe {
        syscall!(SystemCall::SetSignalEntry, argn!(entry), argn!(stack))
    })
}

#[inline(always)]
pub fn sys_ex_sigreturn() -> ! {
    unsafe {
        syscall!(SystemCall::SignalReturn);
    }
    unreachable!();
}

#[inline(always)]
pub fn sys_ex_kill(pid: SignalDestination, signum: Signal) -> Result<(), Errno> {
    Errno::from_syscall_unit(unsafe {
        syscall!(
            SystemCall::SendSignal,
            argn!(isize::from(pid)),
            argn!(signum as u32)
        )
    })
}

#[inline(always)]
pub fn sys_ex_clone(entry: usize, stack: usize, arg: usize) -> Result<usize, Errno> {
    Errno::from_syscall(unsafe {
        syscall!(SystemCall::Clone, argn!(entry), argn!(stack), argn!(arg))
    })
}

#[inline(always)]
pub fn sys_ex_thread_exit(status: ExitCode) -> ! {
    unsafe {
        syscall!(SystemCall::Exit, argn!(i32::from(status)), argn!(1));
    }
    unreachable!();
}

#[inline(always)]
pub fn sys_ex_thread_wait(tid: u32) -> Result<ExitCode, Errno> {
    Errno::from_syscall(unsafe { syscall!(SystemCall::WaitTid, argn!(tid)) })
        .map(|_| ExitCode::from(0))
}

#[inline(always)]
pub fn sys_ex_yield() {
    unsafe {
        syscall!(SystemCall::Yield);
    }
}

#[inline(always)]
pub fn sys_select(
    read_fds: Option<&mut FdSet>,
    write_fds: Option<&mut FdSet>,
    timeout: u64,
) -> Result<usize, Errno> {
    Errno::from_syscall(unsafe {
        syscall!(
            SystemCall::Select,
            argp!(read_fds
                .map(|e| e as *mut _)
                .unwrap_or(core::ptr::null_mut())),
            argp!(write_fds
                .map(|e| e as *mut _)
                .unwrap_or(core::ptr::null_mut())),
            argn!(timeout)
        )
    })
}

#[inline(always)]
pub fn sys_faccessat(
    fd: Option<FileDescriptor>,
    name: &str,
    mode: AccessMode,
    flags: u32,
) -> Result<(), Errno> {
    Errno::from_syscall_unit(unsafe {
        syscall!(
            SystemCall::Access,
            argn!(FileDescriptor::into_i32(fd)),
            argp!(name.as_ptr()),
            argn!(name.len()),
            argn!(mode.bits()),
            argn!(flags)
        )
    })
}

#[inline(always)]
pub fn sys_ex_gettid() -> u32 {
    unsafe { syscall!(SystemCall::GetTid) as u32 }
}

#[inline(always)]
pub fn sys_getpid() -> Pid {
    Pid::try_from(unsafe { syscall!(SystemCall::GetPid) as u32 }).unwrap()
}

#[inline(always)]
pub fn sys_getpgid(pid: Option<Pid>) -> Result<Pid, Errno> {
    Errno::from_syscall(unsafe { syscall!(SystemCall::GetPgid, argn!(Pid::from_option(pid))) })
        .and_then(|e| Pid::try_from(e as u32))
}

#[inline(always)]
pub fn sys_setpgid(pid: Option<Pid>, pgid: Option<Pid>) -> Result<Pid, Errno> {
    Errno::from_syscall(unsafe {
        syscall!(SystemCall::SetPgid, argn!(Pid::from_option(pid)), argn!(Pid::from_option(pgid)))
    })
    .and_then(|e| Pid::try_from(e as u32))
}

#[inline(always)]
pub fn sys_readdir(fd: FileDescriptor, buf: &mut [DirectoryEntry]) -> Result<usize, Errno> {
    Errno::from_syscall(unsafe {
        syscall!(
            SystemCall::ReadDirectory,
            argn!(u32::from(fd)),
            argp!(buf.as_mut_ptr()),
            argn!(buf.len())
        )
    })
}

#[inline(always)]
pub fn sys_getuid() -> UserId {
    UserId::from(unsafe { syscall!(SystemCall::GetUserId) as u32 })
}

#[inline(always)]
pub fn sys_getgid() -> GroupId {
    GroupId::from(unsafe { syscall!(SystemCall::GetGroupId) as u32 })
}

#[inline(always)]
pub fn sys_setsid() -> Result<Pid, Errno> {
    Errno::from_syscall(unsafe { syscall!(SystemCall::SetSid) })
        .and_then(|e| Pid::try_from(e as u32) )
}

#[inline(always)]
pub fn sys_mount(target: &str, options: &MountOptions) -> Result<(), Errno> {
    Errno::from_syscall_unit(unsafe {
        syscall!(
            SystemCall::Mount,
            argp!(target.as_ptr()),
            argn!(target.len()),
            argp!(options as *const _)
        )
    })
}

#[inline(always)]
pub fn sys_dup(src: FileDescriptor, dst: Option<FileDescriptor>) -> Result<FileDescriptor, Errno> {
    Errno::from_syscall(unsafe {
        syscall!(
            SystemCall::DuplicateFd,
            argn!(u32::from(src)),
            argn!(FileDescriptor::into_i32(dst))
        )
    })
    .map(|e| FileDescriptor::from(e as u32))
}

#[inline(always)]
pub fn sys_setuid(uid: UserId) -> Result<(), Errno> {
    Errno::from_syscall_unit(unsafe { syscall!(SystemCall::SetUserId, u32::from(uid) as usize) })
}

#[inline(always)]
pub fn sys_setgid(gid: GroupId) -> Result<(), Errno> {
    Errno::from_syscall_unit(unsafe { syscall!(SystemCall::SetGroupId, u32::from(gid) as usize) })
}

#[inline(always)]
pub fn sys_chdir(path: &str) -> Result<(), Errno> {
    Errno::from_syscall_unit(unsafe {
        syscall!(
            SystemCall::SetCurrentDirectory,
            argp!(path.as_ptr()),
            argn!(path.len())
        )
    })
}

#[inline(always)]
pub fn sys_mmap(
    hint: usize,
    len: usize,
    acc: MemoryAccess,
    flags: MemoryMap,
) -> Result<usize, Errno> {
    Errno::from_syscall(unsafe {
        syscall!(
            SystemCall::MapMemory,
            argn!(hint),
            argn!(len),
            argn!(acc.bits()),
            argn!(flags.bits())
        )
    })
}

#[inline(always)]
pub unsafe fn sys_munmap(addr: usize, len: usize) -> Result<(), Errno> {
    Errno::from_syscall_unit(syscall!(SystemCall::UnmapMemory, argn!(addr), argn!(len)))
}
