//! System call argument ABI helpers

use crate::mem;
use core::mem::size_of;
use syscall::error::Errno;

fn translate(virt: usize) -> Option<usize> {
    let mut res: usize;
    unsafe {
        asm!("at s1e1r, {}; mrs {}, par_el1", in(reg) virt, out(reg) res);
    }
    if res & 1 == 0 {
        Some(res & !(0xFFF | (0xFF << 56)))
    } else {
        None
    }
}

/// Unwraps a slim structure pointer
pub fn validate_user_ptr_struct<'a, T>(base: usize) -> Result<&'a mut T, Errno> {
    let bytes = validate_user_ptr(base, size_of::<T>())?;
    Ok(unsafe { &mut *(bytes.as_mut_ptr() as *mut T) })
}

/// Unwraps an user buffer reference
pub fn validate_user_ptr<'a>(base: usize, len: usize) -> Result<&'a mut [u8], Errno> {
    if base > mem::KERNEL_OFFSET || base + len > mem::KERNEL_OFFSET {
        warnln!(
            "User region refers to kernel memory: base={:#x}, len={:#x}",
            base,
            len
        );
        return Err(Errno::InvalidArgument);
    }

    for i in (base / mem::PAGE_SIZE)..((base + len + mem::PAGE_SIZE - 1) / mem::PAGE_SIZE) {
        if translate(i * mem::PAGE_SIZE).is_none() {
            warnln!(
                "User region refers to unmapped memory: base={:#x}, len={:#x} (page {:#x})",
                base,
                len,
                i * mem::PAGE_SIZE
            );
            return Err(Errno::InvalidArgument);
        }
    }

    Ok(unsafe { core::slice::from_raw_parts_mut(base as *mut u8, len) })
}

/// Unwraps a nullable user buffer reference
pub fn validate_user_ptr_null<'a>(base: usize, len: usize) -> Result<Option<&'a mut [u8]>, Errno> {
    if base == 0 {
        Ok(None)
    } else {
        validate_user_ptr(base, len).map(Some)
    }
}

/// Unwraps user string argument
pub fn validate_user_str<'a>(base: usize, len: usize) -> Result<&'a str, Errno> {
    let bytes = validate_user_ptr(base, len)?;
    core::str::from_utf8(bytes).map_err(|_| {
        warnln!(
            "User string contains invalid UTF-8 characters: base={:#x}, len={:#x}",
            base,
            len
        );
        Errno::InvalidArgument
    })
}
//     if base > mem::KERNEL_OFFSET {
//         warnln!("User string refers to kernel memory: base={:#x}", base);
//         return Err(Errno::InvalidArgument);
//     }
//
//     let base_ptr = base as *const u8;
//     let mut len = 0;
//     let mut page_valid = false;
//     loop {
//         if len == limit {
//             warnln!("User string exceeded limit: base={:#x}", base);
//             return Err(Errno::InvalidArgument);
//         }
//
//         if (base + len) % mem::PAGE_SIZE == 0 {
//             page_valid = false;
//         }
//
//         if !page_valid && translate((base + len) & !0xFFF).is_none() {
//             warnln!(
//                 "User string refers to unmapped memory: base={:#x}, off={:#x}",
//                 base,
//                 len
//             );
//             return Err(Errno::InvalidArgument);
//         }
//
//         page_valid = true;
//
//         let byte = unsafe { *base_ptr.add(len) };
//         if byte == 0 {
//             break;
//         }
//
//         len += 1;
//     }
//
//     let slice = unsafe { core::slice::from_raw_parts(base_ptr, len) };
//     core::str::from_utf8(slice).map_err(|_| {
//         warnln!(
//             "User string contains invalid UTF-8 characters: base={:#x}",
//             base
//         );
//         Errno::InvalidArgument
//     })
// }
