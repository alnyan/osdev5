//! aarch64 common boot logic

use crate::arch::{aarch64::asm::CPACR_EL1, machine};
use crate::dev::{Device, fdt::DeviceTree};
use crate::mem::virt;
use cortex_a::asm::barrier::{self, dsb, isb};
use cortex_a::registers::{DAIF, SCTLR_EL1, VBAR_EL1};
use tock_registers::interfaces::{ReadWriteable, Writeable};

#[no_mangle]
fn __aa64_bsp_main(fdt_base: usize) {
    // Disable FP instruction trapping
    CPACR_EL1.modify(CPACR_EL1::FPEN::TrapNone);

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

    machine::init_board().unwrap();

    let fdt = DeviceTree::from_phys(fdt_base).expect("Failed to obtain a device tree");
    fdt.dump();

    unsafe {
        machine::local_timer().enable().unwrap();
    }

    loop {
        DAIF.modify(DAIF::I::CLEAR);
    }
}

cfg_if! {
    if #[cfg(feature = "mach_orangepi3")] {
        global_asm!(include_str!("uboot.S"));
    } else {
        global_asm!(include_str!("entry.S"));
    }
}
