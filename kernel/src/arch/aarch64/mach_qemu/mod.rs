//! QEMU virt machine

use crate::arch::aarch64::{
    irq::gic::{self, Gic},
    timer::GenericTimer,
};
use crate::dev::timer::TimestampSource;
use crate::dev::{
    irq::{IntController, IntSource},
    pci::{pcie::gpex::GenericPcieHost, PciHostDevice},
    rtc::pl031::Pl031,
    serial::{pl011::Pl011, SerialDevice},
    Device,
};
use crate::mem::phys;
use error::Errno;

pub use gic::IrqNumber;

// TODO extract this from device tree
const LOCAL_TIMER_IRQ: IrqNumber = IrqNumber::new(30);
const UART0_BASE: usize = 0x09000000;
const UART0_IRQ: IrqNumber = IrqNumber::new(33);
const RTC_BASE: usize = 0x09010000;
const RTC_IRQ: IrqNumber = IrqNumber::new(34);
const GICD_BASE: usize = 0x08000000;
const GICC_BASE: usize = 0x08010000;
const ECAM_BASE: usize = 0x4010000000;

const PHYS_BASE: usize = 0x40000000;
const PHYS_SIZE: usize = 0x10000000;

#[allow(missing_docs)]
pub fn init_board_early() -> Result<(), Errno> {
    unsafe {
        // Enable UART early on
        UART0.enable()?;

        phys::init_from_region(PHYS_BASE, PHYS_SIZE);
    }
    Ok(())
}

#[allow(missing_docs)]
pub fn init_board() -> Result<(), Errno> {
    unsafe {
        GIC.enable()?;

        UART0.init_irqs()?;

        RTC.enable()?;
        RTC.init_irqs()?;

        PCIE.enable()?;
        // PCIE.map()?;
    }
    Ok(())
}

/// Returns primary console for this machine
#[inline]
pub fn console() -> &'static impl SerialDevice {
    &UART0
}

/// Returns the timer used as CPU-local periodic IRQ source
#[inline]
pub fn local_timer() -> &'static GenericTimer {
    &LOCAL_TIMER
}

/// Returns CPU's interrupt controller device
#[inline]
pub fn intc() -> &'static impl IntController<IrqNumber = IrqNumber> {
    &GIC
}

static UART0: Pl011 = unsafe { Pl011::new(UART0_BASE, UART0_IRQ) };
static RTC: Pl031 = unsafe { Pl031::new(RTC_BASE, RTC_IRQ) };
static GIC: Gic = unsafe { Gic::new(GICD_BASE, GICC_BASE, LOCAL_TIMER_IRQ) };
static PCIE: GenericPcieHost = unsafe { GenericPcieHost::new(ECAM_BASE, 8) };
static LOCAL_TIMER: GenericTimer = GenericTimer::new(LOCAL_TIMER_IRQ);
