#![allow(missing_docs)]

use crate::dev::fdt::{self, DeviceTree};
use crate::arch::aarch64::cpu;
use crate::mem::{
    self,
    phys::{self, PageUsage},
};
use cortex_a::registers::MPIDR_EL1;
use error::Errno;
use fdt_rs::prelude::*;
use tock_registers::interfaces::Readable;

#[derive(Clone, Copy, Debug)]
pub enum PsciError {
    NotSupported,
    InvalidParameters,
    Denied,
    AlreadyOn,
    OnPending,
    InternalFailure,
    NotPresent,
    Disabled,
    InvalidAddress,
}

const SECONDARY_STACK_PAGES: usize = 4;

pub fn get_cpu_id() -> usize {
    (MPIDR_EL1.get() & 0xF) as usize
}

unsafe fn call_smc(mut x0: usize, x1: usize, x2: usize, x3: usize) -> usize {
    asm!("smc #0", inout("x0") x0, in("x1") x1, in("x2") x2, in("x3") x3);
    x0
}

fn wrap_psci_ok(a: usize) -> Result<(), PsciError> {
    const NOT_SUPPORTED: isize = -1;
    const INVALID_PARAMETERS: isize = -2;
    const DENIED: isize = -3;
    const ALREADY_ON: isize = -4;

    match a as isize {
        0 => Ok(()),
        NOT_SUPPORTED => Err(PsciError::NotSupported),
        INVALID_PARAMETERS => Err(PsciError::InvalidParameters),
        DENIED => Err(PsciError::Denied),
        ALREADY_ON => Err(PsciError::AlreadyOn),
        _ => unimplemented!(),
    }
}

struct Psci {
    use_smc: bool,
}

impl Psci {
    const PSCI_VERSION: usize = 0x84000000;
    const PSCI_CPU_OFF: usize = 0x84000002;
    const PSCI_CPU_ON: usize = 0xC4000003;

    pub const fn new() -> Self {
        Self { use_smc: true }
    }

    unsafe fn call(&self, x0: usize, x1: usize, x2: usize, x3: usize) -> usize {
        if self.use_smc {
            call_smc(x0, x1, x2, x3)
        } else {
            todo!()
        }
    }

    pub unsafe fn cpu_on(
        &self,
        target_cpu: usize,
        entry_point_address: usize,
        context_id: usize,
    ) -> Result<(), PsciError> {
        wrap_psci_ok(self.call(
            Self::PSCI_CPU_ON,
            target_cpu,
            entry_point_address,
            context_id,
        ))
    }
}

pub unsafe fn enable_secondary_cpus(dt: &DeviceTree) {
    extern "C" {
        fn _entry_secondary();
    }

    let cpus = dt.node_by_path("/cpus").unwrap();
    let psci = Psci::new();

    for cpu_node in cpus.children() {
        let reg = fdt::find_prop(cpu_node, "reg").unwrap().u32(0).unwrap();
        if reg == 0 {
            continue;
        }
        infoln!("Enabling cpu{}", reg);
        let stack_pages =
            phys::alloc_contiguous_pages(PageUsage::Kernel, SECONDARY_STACK_PAGES).unwrap();
        let count_old = cpu::count();
        psci.cpu_on(
            reg as usize,
            _entry_secondary as usize - mem::KERNEL_OFFSET,
            mem::virtualize(stack_pages + SECONDARY_STACK_PAGES * mem::PAGE_SIZE),
        )
        .unwrap();
        while cpu::count() == count_old {
            cortex_a::asm::wfe();
        }
        debugln!("Done");
    }
}
