//! System call argument ABI helpers

use crate::mem;
use core::mem::size_of;
use libsys::error::Errno;

// TODO _mut() versions checking whether pages are actually writable

macro_rules! invalid_memory {
    ($($args:tt)+) => {
        warnln!($($args)+);
        #[cfg(feature = "aggressive_syscall")]
        {
            use libsys::signal::Signal;
            use crate::proc::Thread;

            let thread = Thread::current();
            let proc = thread.owner().unwrap();
            proc.enter_fault_signal(thread, Signal::SegmentationFault);
        }
        return Err(Errno::InvalidArgument);
    }
}

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
    validate_user_ptr_struct_option(base).and_then(|e| e.ok_or(Errno::InvalidArgument))
}

pub fn validate_user_ptr_struct_option<'a, T>(base: usize) -> Result<Option<&'a mut T>, Errno> {
    if base == 0 {
        Ok(None)
    } else {
        let bytes = validate_user_ptr(base, size_of::<T>())?;
        Ok(Some(unsafe { &mut *(bytes.as_mut_ptr() as *mut T) }))
    }
}

/// Unwraps an user buffer reference
pub fn validate_user_ptr<'a>(base: usize, len: usize) -> Result<&'a mut [u8], Errno> {
    if base > mem::KERNEL_OFFSET || base + len > mem::KERNEL_OFFSET {
        invalid_memory!(
            "User region refers to kernel memory: base={:#x}, len={:#x}",
            base,
            len
        );
    }

    for i in (base / mem::PAGE_SIZE)..((base + len + mem::PAGE_SIZE - 1) / mem::PAGE_SIZE) {
        if translate(i * mem::PAGE_SIZE).is_none() {
            invalid_memory!(
                "User region refers to unmapped memory: base={:#x}, len={:#x} (page {:#x})",
                base,
                len,
                i * mem::PAGE_SIZE
            );
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
