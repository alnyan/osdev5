use alloc::boxed::Box;
use alloc::vec;
use core::cell::UnsafeCell;
use core::marker::PhantomData;
use libsys::{
    calls::{sys_ex_clone, sys_ex_thread_exit, sys_ex_signal},
    error::Errno,
    proc::ExitCode,
};

use crate::trace;

struct NativeData<F, T>
where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    closure: F,
    stack: usize,
}

pub struct JoinHandle<T> {
    native: u32,
    _pd: PhantomData<T>,
}

pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    let stack = vec![0u8; 8192].leak();

    #[inline(never)]
    extern "C" fn thread_entry<F, T>(data: *mut NativeData<F, T>) -> !
    where
        F: FnOnce() -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        let (stack, len) = {
            // Setup signal handling
            let mut signal_stack = vec![0u8; 8192];

            unsafe {
                sys_ex_signal(
                    crate::_signal_handler as usize,
                    signal_stack.as_mut_ptr() as usize + signal_stack.len(),
                )
                .unwrap();
            }

            let data: Box<NativeData<F, T>> = unsafe { Box::from_raw(data) };

            let res = (data.closure)();

            (data.stack, 8192)
        };

        // TODO free stack
        sys_ex_thread_exit(ExitCode::from(0));
    }

    let native = unsafe {
        let stack = stack.as_mut_ptr() as usize + stack.len();
        let data: *mut NativeData<F, T> = Box::into_raw(Box::new(NativeData { closure: f, stack }));

        sys_ex_clone(thread_entry::<F, T> as usize, stack, data as usize).unwrap() as u32
    };

    JoinHandle {
        native,
        _pd: PhantomData,
    }
}
