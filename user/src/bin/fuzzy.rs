#![feature(asm)]
#![no_std]
#![no_main]

#![allow(unused_macros)]
#![allow(dead_code)]

#[macro_use]
extern crate libusr;

use libusr::sys::{abi::SystemCall, stat::Stat};

static mut STATE: u64 = 0;

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

fn random_set_seed(seed: u64) {
    unsafe { STATE = seed; }
}

fn random_u64() -> u64 {
    let mut x = unsafe { STATE };
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    unsafe {
        STATE = x;
    }
    x
}

fn random_ascii_char() -> u8 {
    ((random_u64() % (0x7F - 0x20)) as u8) + 0x20
}

fn random_str_range(buf: &mut [u8], min: usize, max: usize) -> &str {
    let max = core::cmp::min(buf.len(), max);
    assert!(max > min);
    let len = ((random_u64() as usize) % (max - min)) + min;
    for c in buf[..len].iter_mut() {
        *c = random_ascii_char();
    }
    core::str::from_utf8(&buf[..len]).unwrap()
}

fn random_str(buf: &mut [u8]) -> &str {
    random_str_range(buf, 0, buf.len())
}

fn random_bytes(buf: &mut [u8]) {
    for byte in buf.iter_mut() {
        *byte = (random_u64() & 0xFF) as u8;
    }
}

#[no_mangle]
fn main() -> i32 {
    let seed = libusr::sys::sys_ex_getcputime().unwrap().as_nanos() as u64 / 13;
    println!("Using seed: {:#x}", seed);
    random_set_seed(seed);

    let mut buf = [0; 256];

    // Test sys_ex_getcputime()
    let mut prev_time = libusr::sys::sys_ex_getcputime().unwrap().as_nanos();
    for _ in 0..1000 {
        let t = libusr::sys::sys_ex_getcputime().unwrap().as_nanos();
        assert!(t >= prev_time);
        prev_time = t;
    }

    // Test non-utf8 input fed into syscalls expecting strings
    // let old_signal = signal::set_handler(Signal::InvalidSystemCall, SignalHandler::Ignore);
    for _ in 0..10000 {
        random_bytes(&mut buf);
        let mut stat = Stat::default();

        unsafe {
            syscall!(SystemCall::FileStatus.repr(), (-2i32) as usize, buf.as_mut_ptr() as usize, buf.len(), (&mut stat) as *mut _ as usize);
        }
    }
    // signal::set_handler(Signal::InvalidSystemCall, old_signal);

    0
}
