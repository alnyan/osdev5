use alloc::{boxed::Box, sync::Arc, vec};
use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use libsys::{
    calls::{sys_ex_clone, sys_ex_signal, sys_ex_thread_exit, sys_ex_thread_wait, sys_ex_gettid},
    error::Errno,
    proc::ExitCode,
};
use core::sync::atomic::{AtomicU32, Ordering};
use core::any::Any;
use crate::{trace, signal};
use core::fmt;

struct NativeData<F, T>
where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    closure: F,
    result: ThreadPacket<T>,
    stack: usize,
}

#[derive(Clone)]
pub struct Thread {
    id: u32
}

pub type ThreadResult<T> = Result<T, Box<dyn Any + Send + Sync>>;
pub type ThreadPacket<T> = Arc<UnsafeCell<MaybeUninit<ThreadResult<T>>>>;

pub struct JoinHandle<T> {
    native: u32,
    result: ThreadPacket<T>
}

impl Thread {
    pub const fn id(&self) -> u32 {
        self.id
    }
}

impl fmt::Debug for Thread {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Thread").field("id", &self.id).finish_non_exhaustive()
    }
}

impl<T> JoinHandle<T> {
    pub fn join(self) -> ThreadResult<T> {
        sys_ex_thread_wait(self.native).unwrap();
        unsafe { Arc::try_unwrap(self.result).unwrap().into_inner().assume_init() }
    }
}

unsafe fn init_common(signal_stack_pointer: *mut u8) {
    let tid = sys_ex_gettid();
    asm!("msr tpidr_el0, {}", in(reg) tid);

    // thread::current() should be valid at this point

    sys_ex_signal(
        signal::signal_handler as usize,
        signal_stack_pointer as usize
    )
    .unwrap();
}

pub(crate) unsafe fn init_main() {
    static mut SIGNAL_STACK: [u8; 4096] = [0; 4096];
    init_common(SIGNAL_STACK.as_mut_ptr().add(SIGNAL_STACK.len()))
}

pub fn current() -> Thread {
    let mut id: u32;
    unsafe {
        asm!("mrs {}, tpidr_el0", out(reg) id);
    }

    Thread { id }
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
                init_common(signal_stack.as_mut_ptr().add(signal_stack.len()));
            }

            let data: Box<NativeData<F, T>> = unsafe { Box::from_raw(data) };
            let res = (data.closure)();

            unsafe {
                (&mut *data.result.get()).write(Ok(res));
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
