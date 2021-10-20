//! Thread context

use crate::mem::{
    self,
    phys::{self, PageUsage},
};
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

        stack.setup_common(__aa64_ctx_enter_kernel as usize);

        Self {
            k_sp: stack.sp,

            stack_base_phys: stack.bp,
            stack_page_count: 8,
        }
    }

    /// Constructs a new user-space thread context
    pub fn user(entry: usize, arg: usize, ttbr0: usize, ustack: usize) -> Self {
        let mut stack = Stack::new(8);

        stack.push(entry);
        stack.push(arg);
        stack.push(ttbr0);
        stack.push(ustack);

        stack.setup_common(__aa64_ctx_enter_user as usize);

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

    pub fn setup_common(&mut self, entry: usize) {
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
    fn __aa64_ctx_enter_kernel();
    fn __aa64_ctx_enter_user();
    fn __aa64_ctx_switch(dst: *mut Context, src: *mut Context);
    fn __aa64_ctx_switch_to(dst: *mut Context);
}

global_asm!(include_str!("context.S"));
