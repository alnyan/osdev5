pub fn read_le32(src: &[u8]) -> u32 {
    (src[0] as u32) | ((src[1] as u32) << 8) | ((src[2] as u32) << 16) | ((src[3] as u32) << 24)
}

pub fn read_le16(src: &[u8]) -> u16 {
    (src[0] as u16) | ((src[1] as u16) << 8)
}

/// See memcpy(3p).
///
/// # Safety
///
/// Unsafe: writes to arbitrary memory locations, performs no pointer
/// validation.
#[no_mangle]
pub unsafe extern "C" fn memcpy(dst: *mut u8, src: *const u8, mut len: usize) -> *mut u8 {
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
pub unsafe extern "C" fn memcmp(a: *mut u8, b: *mut u8, len: usize) -> isize {
    let mut off = 0;
    while off != len {
        if *a.add(off) < *b.add(off) {
            return -1;
        }
        if *a.add(off) > *b.add(off) {
            return 1;
        }
        off += 1;
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
    if dst < src {
        for i in 0..len {
            *dst.add(i) = *src.add(i);
        }
    } else {
        for i in 0..len {
            *dst.add(len - (i + 1)) = *src.add(len - (i + 1));
        }
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
