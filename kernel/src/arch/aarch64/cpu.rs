#![allow(missing_docs)]

use crate::proc::Scheduler;
use crate::util::InitOnce;
use core::mem::MaybeUninit;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicUsize, Ordering};
use cortex_a::registers::{MPIDR_EL1, TPIDR_EL1};
use tock_registers::interfaces::{Readable, Writeable};

#[repr(C)]
pub struct Cpu {
    counter: AtomicUsize, // 0x08

    id: usize,
    scheduler: Scheduler,
}

impl Cpu {
    pub fn new(id: usize) -> Self {
        Self {
            counter: AtomicUsize::new(0),

            id,
            scheduler: Scheduler::new(),
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn scheduler(&mut self) -> &Scheduler {
        &self.scheduler
    }

    pub unsafe fn set(&mut self) {
        TPIDR_EL1.set(self as *mut _ as u64);
    }

    pub unsafe fn get() -> &'static mut Self {
        &mut *(TPIDR_EL1.get() as *mut Self)
    }
}

pub unsafe fn cpus() -> impl Iterator<Item = &'static mut Cpu> {
    CPUS[..CPU_COUNT.load(Ordering::Acquire)]
        .iter_mut()
        .map(|c| c.assume_init_mut())
}

pub unsafe fn by_index(idx: usize) -> &'static mut Cpu {
    assert!(idx < CPU_COUNT.load(Ordering::Acquire));
    CPUS[idx].assume_init_mut()
}

pub fn count() -> usize {
    CPU_COUNT.load(Ordering::Acquire)
}

static CPU_COUNT: AtomicUsize = AtomicUsize::new(0);
static mut CPUS: [MaybeUninit<Cpu>; 8] = MaybeUninit::uninit_array();

pub unsafe fn init_self() {
    let cpu_index = CPU_COUNT.load(Ordering::Acquire);
    let mpidr_id = (MPIDR_EL1.get() & 0xF) as usize;

    CPUS[cpu_index].write(Cpu::new(mpidr_id));
    CPUS[cpu_index].assume_init_mut().set();

    CPU_COUNT.store(cpu_index + 1, Ordering::Release);
}
