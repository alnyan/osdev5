//! aarch64 common boot logic

use crate::arch::{aarch64::reg::{CPACR_EL1, CNTKCTL_EL1}, machine};
use crate::dev::{fdt::DeviceTree, irq::IntSource, Device};
//use crate::debug::Level;
use crate::mem::{
    self, heap,
    phys::{self, PageUsage},
    virt,
};
use crate::proc;
use cortex_a::asm::barrier::{self, dsb, isb};
use cortex_a::registers::{SCTLR_EL1, VBAR_EL1};
use tock_registers::interfaces::{ReadWriteable, Writeable};

#[no_mangle]
extern "C" fn __aa64_bsp_main(fdt_base: usize) -> ! {
    // Disable FP instruction trapping
    CPACR_EL1.modify(CPACR_EL1::FPEN::TrapNone);

    // Disable CNTPCT and CNTFRQ trapping from EL0
    CNTKCTL_EL1.modify(CNTKCTL_EL1::EL0PCTEN::SET);

    extern "C" {
        static aa64_el1_vectors: u8;
    }
    unsafe {
        VBAR_EL1.set(&aa64_el1_vectors as *const _ as u64);

        // Setup caching in SCTLR_EL1
        dsb(barrier::SY);
        isb(barrier::SY);

        SCTLR_EL1
            .modify(SCTLR_EL1::I::SET + SCTLR_EL1::SA::SET + SCTLR_EL1::C::SET + SCTLR_EL1::A::SET);

        dsb(barrier::SY);
        isb(barrier::SY);
    }

    // Enable MMU
    virt::enable().expect("Failed to initialize virtual memory");

    // Most basic machine init: initialize proper debug output
    // physical memory
    machine::init_board_early().unwrap();

    // Setup a heap
    unsafe {
        let heap_base_phys = phys::alloc_contiguous_pages(PageUsage::KernelHeap, 4096)
            .expect("Failed to allocate memory for heap");
        let heap_base_virt = mem::virtualize(heap_base_phys);
        heap::init(heap_base_virt, 16 * 1024 * 1024);
    }

    machine::init_board().unwrap();

    let initrd;
    if fdt_base != 0 {
        let fdt = DeviceTree::from_phys(fdt_base + 0xFFFFFF8000000000);
        if let Ok(fdt) = fdt {
            // fdt.dump(Level::Debug);
            initrd = fdt.initrd();
        } else {
            initrd = None;
            errorln!("Failed to init FDT");
        }
    } else {
        initrd = None;
        warnln!("No FDT present");
    }

    infoln!("Machine init finished");

    unsafe {
        machine::local_timer().enable().unwrap();
        machine::local_timer().init_irqs().unwrap();

        proc::enter(initrd);
    }
}

global_asm!(include_str!("macros.S"));
global_asm!(include_str!("uboot.S"));
global_asm!(include_str!("upper.S"));
