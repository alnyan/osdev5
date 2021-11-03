use crate::abi;
use crate::stat::Stat;

macro_rules! syscall {
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
}

#[inline(always)]
pub unsafe fn sys_exit(status: i32) -> ! {
    syscall!(abi::SYS_EXIT, status as usize);
    loop {}
}

#[inline(always)]
pub unsafe fn sys_ex_nanosleep(ns: u64, rem: *mut [u64; 2]) -> i32 {
    syscall!(abi::SYS_EX_NANOSLEEP, ns as usize, rem as usize) as i32
}

#[inline(always)]
pub unsafe fn sys_ex_debug_trace(msg: *const u8, len: usize) -> usize {
    syscall!(abi::SYS_EX_DEBUG_TRACE, msg as usize, len)
}

#[inline(always)]
pub unsafe fn sys_open(pathname: *const u8, mode: u32, flags: u32) -> i32 {
    syscall!(
        abi::SYS_OPEN,
        pathname as usize,
        mode as usize,
        flags as usize
    ) as i32
}

#[inline(always)]
pub unsafe fn sys_read(fd: i32, data: *mut u8, len: usize) -> isize {
    syscall!(abi::SYS_READ, fd as usize, data as usize, len) as isize
}

#[inline(always)]
pub unsafe fn sys_write(fd: i32, data: *const u8, len: usize) -> isize {
    syscall!(abi::SYS_WRITE, fd as usize, data as usize, len) as isize
}

#[inline(always)]
pub unsafe fn sys_fstatat(at: i32, pathname: *const u8, statbuf: *mut Stat, flags: i32) -> i32 {
    syscall!(
        abi::SYS_FSTATAT,
        at as usize,
        pathname as usize,
        statbuf as usize,
        flags as usize
    ) as i32
}
