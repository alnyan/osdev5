//! Memory management and functions module
#![allow(missing_docs)]

pub mod phys;
pub mod virt;
pub mod heap;

/// Virtual offset applied to kernel address space
pub const KERNEL_OFFSET: usize = 0xFFFFFF8000000000;
///
pub fn virtualize(addr: usize) -> usize {
    // TODO remove this function
    addr + KERNEL_OFFSET
}

///
pub fn kernel_end_phys() -> usize {
    extern "C" {
        static __kernel_end: u8;
    }
    unsafe { &__kernel_end as *const _ as usize - KERNEL_OFFSET }
}

///
pub const PAGE_SIZE: usize = 4096;

/// See memcpy(3p).
///
/// # Safety
///
/// Unsafe: writes to arbitrary memory locations, performs no pointer
/// validation.
#[no_mangle]
pub unsafe extern "C" fn memcpy(dst: *mut u8, src: *mut u8, mut len: usize) -> *mut u8 {
    while len != 0 {
        len -= 1;
        *dst.add(len) = *src.add(len);
    }
    dst
}

/// See memcmp(3p).
///
/// # Safety
///
/// Unsafe: performs reads from arbitrary memory locations, performs no
/// pointer validation.
#[no_mangle]
pub unsafe extern "C" fn memcmp(a: *mut u8, b: *mut u8, mut len: usize) -> isize {
    while len != 0 {
        len -= 1;
        if *a.add(len) < *b.add(len) {
            return -1;
        }
        if *a.add(len) > *b.add(len) {
            return 1;
        }
    }
    0
}

/// See memmove(3p)
///
/// # Safety
///
/// Unsafe: writes to arbitrary memory locations, performs no pointer
/// validation.
#[no_mangle]
pub unsafe extern "C" fn memmove(dst: *mut u8, src: *mut u8, len: usize) -> *mut u8 {
    if dst == src {
        return dst;
    }

    if src.add(len) <= dst || dst.add(len) <= src {
        return memcpy(dst, src, len);
    }

    if dst < src {
        let a = src as usize - dst as usize;
        memcpy(dst, src, a);
        memcpy(src, src.add(a), len - a);
    } else {
        let a = dst as usize - src as usize;
        memcpy(dst.add(a), dst, len - a);
        memcpy(dst, src, len);
    }

    dst
}

/// See memset(3p)
///
/// # Safety
///
/// Unsafe: writes to arbitrary memory locations, performs no pointer
/// validation.
#[no_mangle]
pub unsafe extern "C" fn memset(buf: *mut u8, val: u8, mut len: usize) -> *mut u8 {
    while len != 0 {
        len -= 1;
        *buf.add(len) = val;
    }
    buf
}
