#![allow(missing_docs)]

use cortex_a::registers::{TPIDR_EL1, MPIDR_EL1};
use tock_registers::interfaces::{Readable, Writeable};
use core::ptr::null_mut;
use core::mem::MaybeUninit;
use core::sync::atomic::{Ordering, AtomicUsize};
use crate::proc::{Scheduler, process::Context};
use crate::util::InitOnce;

#[repr(C)]
pub struct Cpu {
    active_context: *mut Context,           // 0x00
    counter: AtomicUsize,                   // 0x08

    scheduler: Scheduler
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            active_context: null_mut(),
            counter: AtomicUsize::new(0),

            scheduler: Scheduler::new()
        }
    }

    pub fn tick(&self) {
        self.counter.fetch_add(1, Ordering::SeqCst);
        if self.counter.load(Ordering::Acquire) >= 100 {
            debugln!("{} TICK", MPIDR_EL1.get() & 0xF);
            self.counter.store(0, Ordering::Release);
        }
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
    CPUS[..CPU_COUNT.load(Ordering::Acquire)].iter_mut().map(|c| c.assume_init_mut())
}

pub unsafe fn by_index(idx: usize) -> &'static mut Cpu {
    assert!(idx < CPU_COUNT.load(Ordering::Acquire));
    CPUS[idx].assume_init_mut()
}

pub fn count() -> usize {
    CPU_COUNT.load(Ordering::Acquire)
}

static CPU_COUNT: AtomicUsize = AtomicUsize::new(1);
static mut CPUS: [MaybeUninit<Cpu>; 8] = MaybeUninit::uninit_array();

pub unsafe fn init_bsp() {
    // TODO cpu id different than 0?
    CPUS[0].write(Cpu::new());
    CPUS[0].assume_init_mut().set();
}

pub unsafe fn init_self() {
    let cpu_index = CPU_COUNT.load(Ordering::Acquire);

    CPUS[cpu_index].write(Cpu::new());
    CPUS[cpu_index].assume_init_mut().set();

    CPU_COUNT.store(cpu_index + 1, Ordering::Release);
}
