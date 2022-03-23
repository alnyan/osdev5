#[macro_export]
macro_rules! syscall {
    ($num:expr) => {{
        let mut res: usize = $num.repr();
        core::arch::asm!("syscall",
             inout("rax") res,
             options(nostack));
        res
    }};
    ($num:expr, $a0:expr) => {{
        let mut res: usize = $num.repr();
        core::arch::asm!("syscall",
             inout("rax") res, in("rdi") $a0,
             options(nostack));
        res
    }};
    ($num:expr, $a0:expr, $a1:expr) => {{
        let mut res: usize = $num.repr();
        core::arch::asm!("syscall",
             inout("rax") res, in("rdi") $a0, in("rsi") $a1,
             options(nostack));
        res
    }};
    ($num:expr, $a0:expr, $a1:expr, $a2:expr) => {{
        let mut res: usize = $num.repr();
        core::arch::asm!("syscall",
             inout("rax") res, in("rdi") $a0, in("rsi") $a1,
             in("rdx") $a2, options(nostack));
        res
    }};
    ($num:expr, $a0:expr, $a1:expr, $a2:expr, $a3:expr) => {{
        let mut res: usize = $num.repr();
        core::arch::asm!("syscall",
             inout("rax") res, in("rdi") $a0, in("rsi") $a1,
             in("rdx") $a2, in("r10") $a3, options(nostack));
        res
    }};
    ($num:expr, $a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr) => {{
        let mut res: usize = $num.repr();
        core::arch::asm!("syscall",
             inout("rax") res, in("rdi") $a0, in("rsi") $a1,
             in("rdx") $a2, in("r10") $a3, in("r8") $a4, options(nostack));
        res
    }};
}
