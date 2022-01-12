use crate::arch::x86_64::{self, gdt, idt, intc};
use crate::config::{ConfigKey, CONFIG};
use crate::debug;
use crate::dev::{display::FramebufferInfo, pseudo, Device};
use crate::font;
use crate::fs::{devfs, sysfs};
use crate::mem::{
    self, heap,
    phys::{self, MemoryRegion, PageUsage, ReservedRegion},
    virt,
};
use crate::proc;
use core::arch::{asm, global_asm};
use core::mem::MaybeUninit;
use multiboot2::{BootInformation, MemoryArea};

static mut RESERVED_REGION_MB2: MaybeUninit<ReservedRegion> = MaybeUninit::uninit();

#[no_mangle]
extern "C" fn __x86_64_bsp_main(mb_checksum: u32, mb_info_ptr: u32) -> ! {
    unsafe {
        // Enable SSE support
        asm!(
            r#"
            mov %cr4, %rax
            or $(1 << 9), %rax  // FXSAVE, FXRSTOR
            or $(1 << 10), %rax // OSXMMEXCPT
            mov %rax, %cr4

            mov %cr0, %rax
            and $~(1 << 2), %rax    // Disable EM
            or $(1 << 1), %rax      // Enable MP
            mov %rax, %cr0
        "#,
            options(att_syntax)
        );

        // Setup a proper GDT
        gdt::init();
        idt::init(intc::map_isr_entries);
    }

    virt::enable().expect("Failed to initialize virtual memory");

    let mb_info = unsafe {
        multiboot2::load_with_offset(mb_info_ptr as usize, mem::KERNEL_OFFSET)
            .expect("Failed to load multiboot info structure")
    };

    unsafe {
        let mb_info_page = (mb_info_ptr & !0xFFF) as usize;
        RESERVED_REGION_MB2.write(ReservedRegion::new(
            mb_info_page,
            mb_info_page + ((mb_info.total_size() + 0xFFF) & !0xFFF),
        ));
        phys::reserve("multiboot2", RESERVED_REGION_MB2.as_mut_ptr());

        phys::init_from_iter(
            mb_info
                .memory_map_tag()
                .unwrap()
                .memory_areas()
                .map(|entry| MemoryRegion {
                    start: ((entry.start_address() + 0xFFF) & !0xFFF) as usize,
                    end: (entry.end_address() & !0xFFF) as usize,
                }),
        );
    }

    // Setup a heap
    unsafe {
        let heap_base_phys = phys::alloc_contiguous_pages(PageUsage::KernelHeap, 4096)
            .expect("Failed to allocate memory for heap");
        let heap_base_virt = mem::virtualize(heap_base_phys);
        heap::init(heap_base_virt, 16 * 1024 * 1024);
    }

    let initrd_info = mb_info.module_tags().next().unwrap();
    {
        let mut cfg = CONFIG.lock();

        cfg.set_usize(ConfigKey::InitrdBase, initrd_info.start_address() as usize);
        cfg.set_usize(ConfigKey::InitrdSize, initrd_info.module_size() as usize);
    }

    // Setup hardware
    unsafe {
        x86_64::INTC.enable().ok();
    }

    let fb_info = mb_info.framebuffer_tag().unwrap();
    let virt = mem::virtualize(fb_info.address as usize);
    debugln!(
        "Framebuffer base: phys={:#x}, virt={:#x}",
        fb_info.address,
        virt
    );
    x86_64::DISPLAY.set_framebuffer(FramebufferInfo {
        width: fb_info.width as usize,
        height: fb_info.height as usize,
        phys_base: fb_info.address as usize,
        virt_base: virt,
    });
    font::init();
    debug::set_display(&x86_64::DISPLAY);

    devfs::init();
    sysfs::init();

    devfs::add_named_char_device(&pseudo::ZERO, "zero").unwrap();
    devfs::add_named_char_device(&pseudo::RANDOM, "random").unwrap();

    unsafe {
        proc::enter();
    }
}

global_asm!(include_str!("macros.S"), options(att_syntax));
global_asm!(include_str!("entry.S"), options(att_syntax));
global_asm!(include_str!("upper.S"), options(att_syntax));
