#[no_mangle]
pub unsafe extern "C" fn memcpy(dst: *mut u8, src: *mut u8, mut len: usize) -> *mut u8 {
    while len != 0 {
        len -= 1;
        *dst.add(len) = *src.add(len);
    }
    dst
}

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

#[no_mangle]
pub unsafe extern "C" fn memset(buf: *mut u8, val: u8, mut len: usize) -> *mut u8 {
    while len != 0 {
        len -= 1;
        *buf.add(len) = val;
    }
    buf
}