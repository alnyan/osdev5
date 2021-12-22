use core::arch::{asm, global_asm};
use core::mem::size_of_val;

#[derive(Clone, Copy)]
#[repr(packed)]
pub struct Entry {
    base_lo: u16,
    selector: u16,
    __res0: u8,
    flags: u8,
    base_hi: u16,
    base_ex: u32,
    __res1: u32,
}

#[repr(packed)]
struct Pointer {
    limit: u16,
    offset: usize,
}

pub const SIZE: usize = 256;

impl Entry {
    pub const PRESENT: u8 = 1 << 7;
    pub const INT32: u8 = 0xE;

    pub const fn new(base: usize, selector: u16, flags: u8) -> Self {
        Self {
            base_lo: (base & 0xFFFF) as u16,
            base_hi: ((base >> 16) & 0xFFFF) as u16,
            base_ex: (base >> 32) as u32,
            selector,
            flags,
            __res0: 0,
            __res1: 0,
        }
    }

    const fn empty() -> Self {
        Self {
            base_lo: 0,
            base_hi: 0,
            base_ex: 0,
            selector: 0,
            flags: 0,
            __res0: 0,
            __res1: 0,
        }
    }
}

static mut IDT: [Entry; SIZE] = [Entry::empty(); SIZE];

pub unsafe fn init<F: FnOnce(&mut [Entry; SIZE]) -> ()>(f: F) {
    extern "C" {
        static __x86_64_exception_vectors: [usize; 32];
    }

    for (i, &entry) in __x86_64_exception_vectors.iter().enumerate() {
        IDT[i] = Entry::new(entry, 0x08, Entry::PRESENT | Entry::INT32);
    }

    f(&mut IDT);

    let idtr = Pointer {
        limit: size_of_val(&IDT) as u16 - 1,
        offset: &IDT as *const _ as usize,
    };
    asm!("lidt ({})", in(reg) &idtr, options(att_syntax));
}

global_asm!(include_str!("idt.S"), options(att_syntax));
