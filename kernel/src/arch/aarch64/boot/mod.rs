//! aarch64 common boot logic

use crate::arch::{
    aarch64::reg::{CNTKCTL_EL1, CPACR_EL1},
    machine,
};
use crate::fs::devfs;
use crate::dev::{fdt::{DeviceTree, find_prop}, irq::IntSource, Device};
use error::Errno;
use crate::config::{CONFIG, ConfigKey};
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

fn init_device_tree(fdt_base_phys: usize) -> Result<(), Errno> {
    use fdt_rs::prelude::*;

    let fdt = if fdt_base_phys != 0 {
        DeviceTree::from_phys(fdt_base_phys + 0xFFFFFF8000000000)?
    } else {
        warnln!("No FDT present");
        return Ok(());
    };

    use crate::debug::Level;
    fdt.dump(Level::Debug);

    let mut cfg = CONFIG.lock();

    if let Some(chosen) = fdt.node_by_path("/chosen") {
        if let Some(initrd_start) = find_prop(chosen.clone(), "linux,initrd-start") {
            let initrd_end = find_prop(chosen.clone(), "linux,initrd-end").unwrap();
            let initrd_start = initrd_start.u32(0).unwrap() as usize;
            let initrd_end = initrd_end.u32(0).unwrap() as usize;

            cfg.set_usize(ConfigKey::InitrdBase, initrd_start);
            cfg.set_usize(ConfigKey::InitrdSize, initrd_end - initrd_start);
        }

        if let Some(cmdline) = find_prop(chosen, "bootargs") {
            cfg.set_cmdline(cmdline.str().unwrap());
        }
    }

    Ok(())
}

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

    init_device_tree(fdt_base).expect("Device tree init failed");

    // Setup a heap
    unsafe {
        let heap_base_phys = phys::alloc_contiguous_pages(PageUsage::KernelHeap, 4096)
            .expect("Failed to allocate memory for heap");
        let heap_base_virt = mem::virtualize(heap_base_phys);
        heap::init(heap_base_virt, 16 * 1024 * 1024);
    }

    devfs::init();

    machine::init_board().unwrap();

    debugln!("Config: {:#x?}", CONFIG.lock());
    infoln!("Machine init finished");

    unsafe {
        machine::local_timer().enable().unwrap();
        machine::local_timer().init_irqs().unwrap();

        proc::enter();
    }
}

global_asm!(include_str!("macros.S"));
global_asm!(include_str!("uboot.S"));
global_asm!(include_str!("upper.S"));
