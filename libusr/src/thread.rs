use alloc::{boxed::Box, sync::Arc, vec};
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use libsys::{
    calls::{sys_ex_clone, sys_ex_signal, sys_ex_thread_exit, sys_ex_thread_wait},
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
    result: Arc<UnsafeCell<MaybeUninit<T>>>,
    stack: usize,
}

pub struct JoinHandle<T> {
    native: u32,
    result: Arc<UnsafeCell<MaybeUninit<T>>>,
}

impl<T> JoinHandle<T> {
    pub fn join(self) -> Result<T, ()> {
        sys_ex_thread_wait(self.native).unwrap();
        if let Ok(result) = Arc::try_unwrap(self.result) {
            Ok(unsafe { result.into_inner().assume_init() })
        } else {
            Err(())
        }
    }
}

pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    let stack = vec![0u8; 8192].leak();
    let result = Arc::new(UnsafeCell::new(MaybeUninit::uninit()));

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

            unsafe {
                (&mut *data.result.get()).write(res);
            }

            (data.stack, 8192)
        };

        // TODO free stack
        sys_ex_thread_exit(ExitCode::from(0));
    }

    let native = unsafe {
        let stack = stack.as_mut_ptr() as usize + stack.len();
        let data: *mut NativeData<F, T> = Box::into_raw(Box::new(NativeData {
            closure: f,
            stack,
            result: result.clone(),
        }));

        sys_ex_clone(thread_entry::<F, T> as usize, stack, data as usize).unwrap() as u32
    };

    JoinHandle { native, result }
}
