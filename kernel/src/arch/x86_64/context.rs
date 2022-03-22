use crate::mem::{
    self,
    phys::{self, PageUsage},
};
use crate::arch::platform::ForkFrame;
use core::mem::size_of;
use core::arch::global_asm;

struct Stack {
    bp: usize,
    sp: usize,
}

/// Structure representing thread context
#[repr(C)]
pub struct Context {
    /// Thread's kernel stack pointer
    pub k_sp: usize, // 0x00

    stack_base: usize,
    stack_page_count: usize,
}

impl Context {
    /// Constructs a new kernel-space thread context
    pub fn kernel(entry: usize, arg: usize) -> Self {
        let mut stack = Stack::new(8);

        stack.push(entry);
        stack.push(arg);

        stack.setup_common(__x86_64_ctx_enter_kernel as usize, 0, 0);

        Self {
            k_sp: stack.sp,

            stack_base: stack.bp,
            stack_page_count: 8,
        }
    }

    /// Constructs a new user-space thread context
    pub fn user(entry: usize, arg: usize, cr3: usize, ustack: usize) -> Self {
        let cr3 = cr3 & 0xFFFFFFFF;
        let mut stack = Stack::new(8);
        let stack_top = stack.sp;

        stack.push(entry);
        stack.push(arg);
        stack.push(0);
        stack.push(ustack);

        stack.setup_common(__x86_64_ctx_enter_user as usize, cr3, stack_top);

        Self {
            k_sp: stack.sp,

            stack_base: stack.bp,
            stack_page_count: 8,
        }
    }

    /// Constructs an uninitialized thread context
    pub fn empty() -> Self {
        let stack = Stack::new(8);
        Self {
            k_sp: stack.sp,
            stack_base: stack.bp,
            stack_page_count: 8
        }
    }

    /// Sets up a context for signal entry
    ///
    /// # Safety
    ///
    /// Unsafe: may clobber an already active context
    pub unsafe fn setup_signal_entry(&mut self, entry: usize, arg: usize, cr3: usize, ustack: usize) {
        let cr3 = cr3 & 0xFFFFFFFF;
        let mut stack = Stack::from_base_size(self.stack_base, self.stack_page_count);
        let stack_top = stack.sp;

        stack.push(entry);
        stack.push(arg);
        stack.push(0);
        stack.push(ustack);

        stack.setup_common(__x86_64_ctx_enter_user as usize, cr3, stack_top);

        self.k_sp = stack.sp;
    }

    /// Clones a process context from given `frame`
    pub fn fork(frame: &ForkFrame, cr3: usize) -> Self {
        let mut stack = Stack::new(8);
        let stack_top = stack.sp;

        stack.push(frame.saved_rip);
        stack.push(frame.saved_rsp);

        stack.push(frame.x[6]);     // rax
        stack.push(frame.x[5]);     // r9
        stack.push(frame.x[4]);     // r8
        stack.push(frame.x[3]);     // r10
        stack.push(frame.x[2]);     // rdx
        stack.push(frame.x[1]);     // rsi
        stack.push(frame.x[0]);     // rdi

        // Setup common
        stack.push(__x86_64_ctx_enter_from_fork as usize);   // return address
        stack.push(stack_top);       // gs_base
        stack.push(cr3);
        stack.push(frame.x[9]);       // r15
        stack.push(frame.x[10]);      // r14
        stack.push(frame.x[11]);      // r13
        stack.push(frame.x[12]);      // r12
        stack.push(frame.x[7]);       // rbx
        stack.push(frame.x[8]);       // rbp

        Self {
            k_sp: stack.sp,

            stack_base: stack.bp,
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
        __x86_64_ctx_switch_to(self);
        panic!("This code should not run");
    }

    /// Performs context switch from `self` to `to`.
    ///
    /// # Safety
    ///
    /// Unsafe: does not check if `self` is actually an active context.
    pub unsafe extern "C" fn switch(&mut self, to: &mut Context) {
        __x86_64_ctx_switch(to, self);
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

    pub unsafe fn from_base_size(bp: usize, page_count: usize) -> Stack {
        Stack {
            bp,
            sp: bp + page_count * mem::PAGE_SIZE
        }
    }

    pub fn setup_common(&mut self, entry: usize, cr3: usize, tss_rsp0: usize) {
        self.push(entry);   // return address
        self.push(tss_rsp0);       // gs_base
        self.push(cr3);
        self.push(0);       // r15
        self.push(0);       // r14
        self.push(0);       // r13
        self.push(0);       // r12
        self.push(0);       // rbx
        self.push(0);       // rbp
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
    fn __x86_64_ctx_enter_from_fork();
    fn __x86_64_ctx_enter_kernel();
    fn __x86_64_ctx_enter_user();
    fn __x86_64_ctx_switch(dst: *mut Context, src: *mut Context);
    fn __x86_64_ctx_switch_to(dst: *mut Context);
}

global_asm!(include_str!("context.S"), options(att_syntax));
