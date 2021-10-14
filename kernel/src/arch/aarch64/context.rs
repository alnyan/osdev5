#![allow(missing_docs)]

use crate::mem::{self, phys::{self, PageUsage}};
use core::mem::size_of;

struct Stack {
    bp: usize,
    sp: usize,
}

#[repr(C)]
pub struct Context {
    pub k_sp: usize,

    stack_base_phys: usize,
    stack_page_count: usize,
}

impl Context {
    pub fn kernel(entry: usize, arg: usize) -> Self {
        let mut stack = Stack::new(4);

        stack.push(entry);
        stack.push(arg);

        stack.push(__aa64_ctx_enter_kernel as usize); // x30/lr
        stack.push(0);      // xzr
        stack.push(0);      // x29
        stack.push(0);      // x27
        stack.push(0);      // x26
        stack.push(0);      // x25
        stack.push(0);      // x24
        stack.push(0);      // x23
        stack.push(0);      // x22
        stack.push(0);      // x21
        stack.push(0);      // x20
        stack.push(0);      // x19

        Self {
            k_sp: stack.sp,

            stack_base_phys: stack.bp,
            stack_page_count: 4
        }
    }

    pub unsafe fn enter(&mut self) -> ! {
        __aa64_ctx_switch_to(self);
        panic!("This code should not run");
    }

    pub unsafe fn switch(&mut self, to: &mut Context) {
        __aa64_ctx_switch(to, self);
    }
}

impl Stack {
    pub fn new(page_count: usize) -> Stack {
        let phys = phys::alloc_contiguous_pages(PageUsage::Kernel, page_count).unwrap();
        let bp = mem::virtualize(phys);
        Stack {
            bp,
            sp: bp + page_count * mem::PAGE_SIZE
        }
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
    fn __aa64_ctx_switch(dst: *mut Context, src: *mut Context);
    fn __aa64_ctx_switch_to(dst: *mut Context);
}

global_asm!(include_str!("context.S"));
