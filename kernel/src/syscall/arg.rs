//! System call argument ABI helpers

use crate::mem;
use core::alloc::Layout;
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

#[inline(always)]
fn is_el0_accessible(virt: usize, write: bool) -> bool {
    let mut res: usize;
    unsafe {
        if write {
            asm!("at s1e0w, {}; mrs {}, par_el1", in(reg) virt, out(reg) res);
        } else {
            asm!("at s1e0r, {}; mrs {}, par_el1", in(reg) virt, out(reg) res);
        }
    }
    res & 1 == 0
}

pub fn struct_ref<'a, T>(base: usize) -> Result<&'a T, Errno> {
    let layout = Layout::new::<T>();
    if base % layout.align() != 0 {
        invalid_memory!(
            "Structure pointer is misaligned: base={:#x}, expected {:?}",
            base,
            layout
        );
    }
    let bytes = buf_ref(base, layout.size())?;
    Ok(unsafe { &*(bytes.as_ptr() as *const T) })
}

pub fn struct_mut<'a, T>(base: usize) -> Result<&'a mut T, Errno> {
    let layout = Layout::new::<T>();
    if base % layout.align() != 0 {
        invalid_memory!(
            "Structure pointer is misaligned: base={:#x}, expected {:?}",
            base,
            layout
        );
    }
    let bytes = buf_mut(base, layout.size())?;
    Ok(unsafe { &mut *(bytes.as_mut_ptr() as *mut T) })
}

pub fn option_struct_ref<'a, T>(base: usize) -> Result<Option<&'a T>, Errno> {
    if base == 0 {
        Ok(None)
    } else {
        struct_ref(base).map(Some)
    }
}

pub fn option_struct_mut<'a, T>(base: usize) -> Result<Option<&'a mut T>, Errno> {
    if base == 0 {
        Ok(None)
    } else {
        struct_mut(base).map(Some)
    }
}

fn validate_ptr(base: usize, len: usize, writable: bool) -> Result<(), Errno> {
    if base > mem::KERNEL_OFFSET || base + len > mem::KERNEL_OFFSET {
        invalid_memory!(
            "User region refers to kernel memory: base={:#x}, len={:#x}",
            base,
            len
        );
    }

    for i in (base / mem::PAGE_SIZE)..((base + len + mem::PAGE_SIZE - 1) / mem::PAGE_SIZE) {
        if !is_el0_accessible(i * mem::PAGE_SIZE, writable) {
            invalid_memory!(
                "User region refers to inaccessible/unmapped memory: base={:#x}, len={:#x} (page {:#x})",
                base,
                len,
                i * mem::PAGE_SIZE
            );
        }
    }

    Ok(())
}

pub fn buf_ref<'a>(base: usize, len: usize) -> Result<&'a [u8], Errno> {
    validate_ptr(base, len, false)?;
    Ok(unsafe { core::slice::from_raw_parts(base as *const u8, len) })
}

pub fn buf_mut<'a>(base: usize, len: usize) -> Result<&'a mut [u8], Errno> {
    validate_ptr(base, len, true)?;
    Ok(unsafe { core::slice::from_raw_parts_mut(base as *mut u8, len) })
}

pub fn option_buf_ref<'a>(base: usize, len: usize) -> Result<Option<&'a [u8]>, Errno> {
    if base == 0 {
        Ok(None)
    } else {
        buf_ref(base, len).map(Some)
    }
}

pub fn option_buf_mut<'a>(base: usize, len: usize) -> Result<Option<&'a mut [u8]>, Errno> {
    if base == 0 {
        Ok(None)
    } else {
        buf_mut(base, len).map(Some)
    }
}

/// Unwraps user string argument
pub fn str_ref<'a>(base: usize, len: usize) -> Result<&'a str, Errno> {
    let bytes = buf_ref(base, len)?;
    core::str::from_utf8(bytes).map_err(|_| {
        warnln!(
            "User string contains invalid UTF-8 characters: base={:#x}, len={:#x}",
            base,
            len
        );
        Errno::InvalidArgument
    })
}
