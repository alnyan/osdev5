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
}

#[inline(always)]
pub unsafe fn sys_exit(status: i32) -> ! {
    syscall!(1, status as usize);
    loop {}
}

#[inline(always)]
pub unsafe fn sys_ex_debug_trace(msg: *const u8, len: usize) -> usize {
    syscall!(120, msg as usize, len)
}

#[inline(always)]
pub unsafe fn sys_open(pathname: *const u8, mode: u32, flags: u32) -> i32 {
    syscall!(2, pathname as usize, mode as usize, flags as usize) as i32
}

#[inline(always)]
pub unsafe fn sys_read(fd: i32, data: *mut u8, len: usize) -> isize {
    syscall!(3, fd as usize, data as usize, len) as isize
}

#[inline(always)]
pub unsafe fn sys_write(fd: i32, data: *const u8, len: usize) -> isize {
    syscall!(4, fd as usize, data as usize, len) as isize
}
