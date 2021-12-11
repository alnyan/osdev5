use core::arch::asm;
use core::mem::size_of_val;

#[repr(packed)]
struct Entry {
    limit_lo: u16,
    base_lo: u16,
    base_mi: u8,
    access: u8,
    flags: u8,
    base_hi: u8,
}

#[repr(packed)]
struct Tss {
    __res0: u32,
    rsp0: u64,
    rsp1: u64,
    rsp2: u64,
    __res1: u32,
    ist1: u64,
    ist2: u64,
    ist3: u64,
    ist4: u64,
    ist5: u64,
    ist6: u64,
    ist7: u64,
    __res2: u64,
    __res3: u16,
    iopb_base: u16,
}

#[repr(packed)]
struct Pointer {
    size: u16,
    offset: usize,
}

impl Entry {
    const FLAG_LONG: u8 = 1 << 5;
    const ACC_PRESENT: u8 = 1 << 7;
    const ACC_SYSTEM: u8 = 1 << 4;
    const ACC_EXECUTE: u8 = 1 << 3;
    const ACC_WRITE: u8 = 1 << 1;
    const ACC_RING3: u8 = 3 << 5;
    const ACC_ACCESS: u8 = 1 << 0;

    const fn new(base: u32, limit: u32, flags: u8, access: u8) -> Self {
        Self {
            base_lo: (base & 0xFFFF) as u16,
            base_mi: ((base >> 16) & 0xFF) as u8,
            base_hi: ((base >> 24) & 0xFF) as u8,
            access,
            flags: (flags & 0xF0) | (((limit >> 16) & 0xF) as u8),
            limit_lo: (limit & 0xFFFF) as u16,
        }
    }

    const fn null() -> Self {
        Self {
            base_lo: 0,
            base_mi: 0,
            base_hi: 0,
            access: 0,
            flags: 0,
            limit_lo: 0,
        }
    }
}

impl Tss {
    const fn new() -> Self {
        Self {
            __res0: 0,
            rsp0: 0,
            rsp1: 0,
            rsp2: 0,
            __res1: 0,
            ist1: 0,
            ist2: 0,
            ist3: 0,
            ist4: 0,
            ist5: 0,
            ist6: 0,
            ist7: 0,
            __res2: 0,
            __res3: 0,
            iopb_base: 0,
        }
    }
}

const SIZE: usize = 7;
static mut TSS: Tss = Tss::new();
static mut GDT: [Entry; SIZE] = [
    Entry::null(),
    Entry::new(
        0,
        0,
        Entry::FLAG_LONG,
        Entry::ACC_PRESENT | Entry::ACC_SYSTEM | Entry::ACC_EXECUTE,
    ),
    Entry::new(
        0,
        0,
        0,
        Entry::ACC_PRESENT | Entry::ACC_SYSTEM | Entry::ACC_WRITE,
    ),
    Entry::new(
        0,
        0,
        0,
        Entry::ACC_PRESENT | Entry::ACC_SYSTEM | Entry::ACC_RING3 | Entry::ACC_WRITE,
    ),
    Entry::new(
        0,
        0,
        Entry::FLAG_LONG,
        Entry::ACC_PRESENT | Entry::ACC_SYSTEM | Entry::ACC_RING3 | Entry::ACC_EXECUTE,
    ),
    Entry::null(),
    Entry::null(),
];

pub unsafe fn init() {
    let tss_addr = &TSS as *const _ as usize;

    GDT[5] = Entry::new(
        (tss_addr & 0xFFFFFFFF) as u32,
        size_of_val(&TSS) as u32 - 1,
        Entry::FLAG_LONG,
        Entry::ACC_ACCESS | Entry::ACC_PRESENT | Entry::ACC_EXECUTE,
    );
    core::ptr::write(&mut GDT[6] as *mut _ as *mut u64, (tss_addr >> 32) as u64);

    let gdtr = Pointer {
        size: size_of_val(&GDT) as u16 - 1,
        offset: &GDT as *const _ as usize,
    };
    asm!(r#"
        lgdt ({})

        mov $0x28, %ax
        ltr %ax
    "#, in(reg) &gdtr, options(att_syntax));
}
