use crate::abi;
use crate::{
    ioctl::IoctlCmd,
    signal::Signal,
    stat::{FileMode, OpenFlags, Stat},
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
pub unsafe fn sys_exit(status: i32) -> ! {
    syscall!(abi::SYS_EXIT, argn!(status));
    unreachable!();
}

/// # Safety
///
/// System call
#[inline(always)]
pub unsafe fn sys_close(fd: i32) -> i32 {
    syscall!(abi::SYS_CLOSE, argn!(fd)) as i32
}

/// # Safety
///
/// System call
#[inline(always)]
pub unsafe fn sys_ex_nanosleep(ns: u64, rem: &mut [u64; 2]) -> i32 {
    syscall!(abi::SYS_EX_NANOSLEEP, argn!(ns), argp!(rem.as_mut_ptr())) as i32
}

/// # Safety
///
/// System call
#[inline(always)]
pub unsafe fn sys_ex_debug_trace(msg: &[u8]) -> usize {
    syscall!(
        abi::SYS_EX_DEBUG_TRACE,
        argp!(msg.as_ptr()),
        argn!(msg.len())
    )
}

/// # Safety
///
/// System call
#[inline(always)]
pub unsafe fn sys_openat(at: i32, pathname: &str, mode: FileMode, flags: OpenFlags) -> i32 {
    syscall!(
        abi::SYS_OPENAT,
        argn!(at),
        argp!(pathname.as_ptr()),
        argn!(pathname.len()),
        argn!(mode.bits()),
        argn!(flags.bits())
    ) as i32
}

/// # Safety
///
/// System call
#[inline(always)]
pub unsafe fn sys_read(fd: i32, data: &mut [u8]) -> isize {
    syscall!(
        abi::SYS_READ,
        argn!(fd),
        argp!(data.as_mut_ptr()),
        argn!(data.len())
    ) as isize
}

/// # Safety
///
/// System call
#[inline(always)]
pub unsafe fn sys_write(fd: i32, data: &[u8]) -> isize {
    syscall!(
        abi::SYS_WRITE,
        argn!(fd),
        argp!(data.as_ptr()),
        argn!(data.len())
    ) as isize
}

/// # Safety
///
/// System call
#[inline(always)]
pub unsafe fn sys_fstatat(at: i32, pathname: &str, statbuf: &mut Stat, flags: u32) -> i32 {
    syscall!(
        abi::SYS_FSTATAT,
        argn!(at),
        argp!(pathname.as_ptr()),
        argn!(pathname.len()),
        argp!(statbuf as *mut Stat),
        argn!(flags)
    ) as i32
}

/// # Safety
///
/// System call
#[inline(always)]
pub unsafe fn sys_fork() -> i32 {
    syscall!(abi::SYS_FORK) as i32
}

/// # Safety
///
/// System call
#[inline(always)]
pub unsafe fn sys_execve(pathname: &str) -> i32 {
    syscall!(
        abi::SYS_EXECVE,
        argp!(pathname.as_ptr()),
        argn!(pathname.len())
    ) as i32
}

/// # Safety
///
/// System call
#[inline(always)]
pub unsafe fn sys_waitpid(pid: u32, status: &mut i32) -> i32 {
    syscall!(abi::SYS_WAITPID, argn!(pid), argp!(status as *mut i32)) as i32
}

/// # Safety
///
/// System call
#[inline(always)]
pub unsafe fn sys_ioctl(fd: u32, cmd: IoctlCmd, ptr: usize, len: usize) -> isize {
    syscall!(
        abi::SYS_IOCTL,
        argn!(fd),
        argn!(cmd),
        argn!(ptr),
        argn!(len)
    ) as isize
}

#[inline(always)]
pub unsafe fn sys_ex_signal(entry: usize, stack: usize) {
    syscall!(abi::SYS_EX_SIGNAL, argn!(entry), argn!(stack));
}

#[inline(always)]
pub unsafe fn sys_ex_sigreturn() -> ! {
    syscall!(abi::SYS_EX_SIGRETURN);
    unreachable!();
}

#[inline(always)]
pub unsafe fn sys_ex_kill(pid: u32, signum: Signal) -> i32 {
    syscall!(abi::SYS_EX_KILL, argn!(pid), argn!(signum as u32)) as i32
}