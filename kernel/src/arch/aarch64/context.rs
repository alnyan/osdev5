//! Thread context

use crate::mem::{
    self,
    phys::{self, PageUsage},
};
use crate::arch::aarch64::exception::ExceptionFrame;
use core::mem::size_of;

struct Stack {
    bp: usize,
    sp: usize,
}

/// Structure representing thread context
#[repr(C)]
pub struct Context {
    /// Thread's kernel stack pointer
    pub k_sp: usize, // 0x00

    stack_base_phys: usize,
    stack_page_count: usize,
}

impl Context {
    /// Constructs a new kernel-space thread context
    pub fn kernel(entry: usize, arg: usize) -> Self {
        let mut stack = Stack::new(8);

        stack.push(entry);
        stack.push(arg);

        stack.setup_common(__aa64_ctx_enter_kernel as usize, 0);

        Self {
            k_sp: stack.sp,

            stack_base_phys: stack.bp,
            stack_page_count: 8,
        }
    }

    pub fn fork(frame: &ExceptionFrame, ttbr0: usize) -> Self {
        let mut stack = Stack::new(8);

        stack.push(frame.x[18]);
        stack.push(frame.x[17]);
        stack.push(frame.x[16]);
        stack.push(frame.x[15]);
        stack.push(frame.x[14]);
        stack.push(frame.x[13]);
        stack.push(frame.x[12]);
        stack.push(frame.x[11]);
        stack.push(frame.x[10]);
        stack.push(frame.x[9]);
        stack.push(frame.x[8]);
        stack.push(frame.x[7]);
        stack.push(frame.x[6]);
        stack.push(frame.x[5]);
        stack.push(frame.x[4]);
        stack.push(frame.x[3]);
        stack.push(frame.x[2]);
        stack.push(frame.x[1]);

        stack.push(frame.elr_el1 as usize);
        stack.push(frame.sp_el0 as usize);

        // Setup common
        stack.push(0);
        stack.push(ttbr0);
        stack.push(__aa64_ctx_enter_from_fork as usize); // x30/lr
        stack.push(frame.x[29]); // x29
        stack.push(frame.x[28]); // x28
        stack.push(frame.x[27]); // x27
        stack.push(frame.x[26]); // x26
        stack.push(frame.x[25]); // x25
        stack.push(frame.x[24]); // x24
        stack.push(frame.x[23]); // x23
        stack.push(frame.x[22]); // x22
        stack.push(frame.x[21]); // x21
        stack.push(frame.x[20]); // x20
        stack.push(frame.x[19]); // x19

        Self {
            k_sp: stack.sp,

            stack_base_phys: stack.bp,
            stack_page_count: 8
        }
    }

    /// Constructs a new user-space thread context
    pub fn user(entry: usize, arg: usize, ttbr0: usize, ustack: usize) -> Self {
        let mut stack = Stack::new(8);

        stack.push(entry);
        stack.push(arg);
        stack.push(/* ttbr0 */ 0);
        stack.push(ustack);

        stack.setup_common(__aa64_ctx_enter_user as usize, ttbr0);

        Self {
            k_sp: stack.sp,

            stack_base_phys: stack.bp,
            stack_page_count: 8,
        }
    }

    /// Performs initial thread entry
    ///
    /// # Safety
    ///
    /// Unsafe: does not check if any context has already been activated
    /// before, so must only be called once.
    pub unsafe extern "C" fn enter(&mut self) -> ! {
        __aa64_ctx_switch_to(self);
        panic!("This code should not run");
    }

    /// Performs context switch from `self` to `to`.
    ///
    /// # Safety
    ///
    /// Unsafe: does not check if `self` is actually an active context.
    pub unsafe extern "C" fn switch(&mut self, to: &mut Context) {
        __aa64_ctx_switch(to, self);
    }
}

impl Stack {
    pub fn new(page_count: usize) -> Stack {
        let phys = phys::alloc_contiguous_pages(PageUsage::Kernel, page_count).unwrap();
        let bp = mem::virtualize(phys);
        Stack {
            bp,
            sp: bp + page_count * mem::PAGE_SIZE,
        }
    }

    pub fn setup_common(&mut self, entry: usize, ttbr: usize) {
        self.push(0);
        self.push(ttbr);
        self.push(entry); // x30/lr
        self.push(0); // x29
        self.push(0); // x28
        self.push(0); // x27
        self.push(0); // x26
        self.push(0); // x25
        self.push(0); // x24
        self.push(0); // x23
        self.push(0); // x22
        self.push(0); // x21
        self.push(0); // x20
        self.push(0); // x19
    }

    pub fn push(&mut self, value: usize) {
        if self.bp == self.sp {
            panic!("Stack overflow");
        }

        self.sp -= size_of::<usize>();
        unsafe {
            *(self.sp as *mut usize) = value;
        }
    }
}

extern "C" {
    fn __aa64_ctx_enter_from_fork();
    fn __aa64_ctx_enter_kernel();
    fn __aa64_ctx_enter_user();
    fn __aa64_ctx_switch(dst: *mut Context, src: *mut Context);
    fn __aa64_ctx_switch_to(dst: *mut Context);
}

global_asm!(include_str!("context.S"));
