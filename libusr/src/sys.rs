macro_rules! syscall {
    ($num:expr, $a0:expr) => {{
        let mut res: usize = $a0;
        unsafe {
            asm!("svc #0", inout("x0") res, in("x8") $num, options(nostack));
        }
        res
    }};
    ($num:expr, $a0:expr, $a1:expr) => {{
        let mut res: usize = $a0;
        unsafe {
            asm!("svc #0", inout("x0") res, in("x1") $a1, in("x8") $num, options(nostack));
        }
        res
    }};
}

#[inline(always)]
pub fn sys_exit(status: i32) -> ! {
    syscall!(1, status as usize);
    loop {}
}

#[inline(always)]
pub fn sys_ex_debug_trace(msg: *const u8, len: usize) -> usize {
    syscall!(120, msg as usize, len)
}
