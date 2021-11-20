use crate::abi;
use crate::{
    error::Errno,
    ioctl::IoctlCmd,
    proc::{ExitCode, Pid},
    signal::{Signal, SignalDestination},
    stat::{AccessMode, FdSet, FileDescriptor, FileMode, OpenFlags, Stat},
};

// TODO document the syscall ABI

macro_rules! syscall {
    ($num:expr) => {{
        let mut res: usize;
        asm!("svc #0", out("x0") res, in("x8") $num, options(nostack));
        res
    }};
    ($num:expr, $a0:expr) => {{
        let mut res: usize = $a0;
        asm!("svc #0",
             inout("x0") res,
             in("x8") $num, options(nostack));
        res
    }};
    ($num:expr, $a0:expr, $a1:expr) => {{
        let mut res: usize = $a0;
        asm!("svc #0",
             inout("x0") res, in("x1") $a1,
             in("x8") $num, options(nostack));
        res
    }};
    ($num:expr, $a0:expr, $a1:expr, $a2:expr) => {{
        let mut res: usize = $a0;
        asm!("svc #0",
             inout("x0") res, in("x1") $a1, in("x2") $a2,
             in("x8") $num, options(nostack));
        res
    }};
    ($num:expr, $a0:expr, $a1:expr, $a2:expr, $a3:expr) => {{
        let mut res: usize = $a0;
        asm!("svc #0",
             inout("x0") res, in("x1") $a1, in("x2") $a2,
             in("x3") $a3, in("x8") $num, options(nostack));
        res
    }};
    ($num:expr, $a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr) => {{
        let mut res: usize = $a0;
        asm!("svc #0",
             inout("x0") res, in("x1") $a1, in("x2") $a2,
             in("x3") $a3, in("x4") $a4, in("x8") $num, options(nostack));
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
        syscall!(abi::SYS_EXIT, argn!(i32::from(code)));
    }
    unreachable!();
}

/// # Safety
///
/// System call
#[inline(always)]
pub fn sys_close(fd: FileDescriptor) -> Result<(), Errno> {
    Errno::from_syscall_unit(unsafe { syscall!(abi::SYS_CLOSE, argn!(u32::from(fd))) })
}

/// # Safety
///
/// System call
#[inline(always)]
pub fn sys_ex_nanosleep(ns: u64, rem: &mut [u64; 2]) -> Result<(), Errno> {
    Errno::from_syscall_unit(unsafe {
        syscall!(abi::SYS_EX_NANOSLEEP, argn!(ns), argp!(rem.as_mut_ptr()))
    })
}

/// # Safety
///
/// System call
#[inline(always)]
pub fn sys_ex_debug_trace(msg: &[u8]) -> Result<(), Errno> {
    Errno::from_syscall_unit(unsafe {
        syscall!(
            abi::SYS_EX_DEBUG_TRACE,
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
            abi::SYS_OPENAT,
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
            abi::SYS_READ,
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
            abi::SYS_WRITE,
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
            abi::SYS_FSTATAT,
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
pub fn sys_fork() -> Result<Option<Pid>, Errno> {
    Errno::from_syscall(unsafe { syscall!(abi::SYS_FORK) }).map(|res| {
        if res != 0 {
            Some(unsafe { Pid::from_raw(res as u32) })
        } else {
            None
        }
    })
}

/// # Safety
///
/// System call
#[inline(always)]
pub fn sys_execve(pathname: &str) -> Result<(), Errno> {
    Errno::from_syscall_unit(unsafe {
        syscall!(
            abi::SYS_EXECVE,
            argp!(pathname.as_ptr()),
            argn!(pathname.len())
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
            abi::SYS_WAITPID,
            argn!(pid.value()),
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
            abi::SYS_IOCTL,
            argn!(u32::from(fd)),
            argn!(cmd),
            argn!(ptr),
            argn!(len)
        )
    })
}

#[inline(always)]
pub fn sys_ex_signal(entry: usize, stack: usize) -> Result<(), Errno> {
    Errno::from_syscall_unit(unsafe { syscall!(abi::SYS_EX_SIGNAL, argn!(entry), argn!(stack)) })
}

#[inline(always)]
pub fn sys_ex_sigreturn() -> ! {
    unsafe {
        syscall!(abi::SYS_EX_SIGRETURN);
    }
    unreachable!();
}

#[inline(always)]
pub fn sys_ex_kill(pid: SignalDestination, signum: Signal) -> Result<(), Errno> {
    Errno::from_syscall_unit(unsafe {
        syscall!(
            abi::SYS_EX_KILL,
            argn!(isize::from(pid)),
            argn!(signum as u32)
        )
    })
}

#[inline(always)]
pub fn sys_ex_clone(entry: usize, stack: usize, arg: usize) -> Result<usize, Errno> {
    Errno::from_syscall(unsafe {
        syscall!(abi::SYS_EX_CLONE, argn!(entry), argn!(stack), argn!(arg))
    })
}

#[inline(always)]
pub fn sys_ex_thread_exit(status: ExitCode) -> ! {
    unsafe {
        syscall!(abi::SYS_EX_THREAD_EXIT, argn!(i32::from(status)));
    }
    unreachable!();
}

#[inline(always)]
pub fn sys_ex_thread_wait(tid: u32) -> Result<ExitCode, Errno> {
    Errno::from_syscall(unsafe { syscall!(abi::SYS_EX_THREAD_WAIT, argn!(tid)) })
        .map(|_| ExitCode::from(0))
}

#[inline(always)]
pub fn sys_ex_yield() {
    unsafe {
        syscall!(abi::SYS_EX_YIELD);
    }
}

#[inline(always)]
pub fn sys_ex_undefined() {
    unsafe {
        syscall!(0);
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
            abi::SYS_SELECT,
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
            abi::SYS_FACCESSAT,
            argn!(FileDescriptor::into_i32(fd)),
            argp!(name.as_ptr()),
            argn!(name.len()),
            argn!(mode.bits()),
            argn!(flags)
        )
    })
}
