#![allow(missing_docs)]

use crate::arch::aarch64::timer::GenericTimer;
use crate::dev::{Device, serial::{SerialDevice, pl011::Pl011}, irq::IntSource};
use crate::mem::phys;
use error::Errno;

pub mod irqchip;
pub use irqchip::{IrqNumber, Bcm283xIrqchip};
pub mod emmc;
pub use emmc::MassMediaController;
pub mod mailbox;
pub use mailbox::Bcm283xMailbox;

const UART_BASE: usize = 0x3F201000;
const EMMC_BASE: usize = 0x3F300000;
const BCM_MBOX_BASE: usize = 0x3F00B880;
const UART_IRQ: IrqNumber = IrqNumber::bcm_irq(57);
const LOCAL_TIMER_IRQ: IrqNumber = IrqNumber::qa7_irq(1);

pub fn init_board_early() -> Result<(), Errno> {
    unsafe {
        UART.enable()?;
        BCM_MBOX.enable()?;

        let memory = BCM_MBOX.memory_split()?;
        infoln!("Memory split: {:#x}", memory);

        phys::init_from_region(0, memory);
    }
    Ok(())
}

pub fn init_board() -> Result<(), Errno> {
    unsafe {
        IRQCHIP.enable()?;
        UART.init_irqs()?;

        EMMC.enable()?;
    }
    Ok(())
}

#[inline]
pub fn intc() -> &'static Bcm283xIrqchip {
    &IRQCHIP
}

/// Returns primary console for this machine
#[inline]
pub fn console() -> &'static impl SerialDevice {
    &UART
}

/// Returns the timer used as CPU-local periodic IRQ source
#[inline]
pub fn local_timer() -> &'static GenericTimer {
    &LOCAL_TIMER
}

static IRQCHIP: Bcm283xIrqchip = Bcm283xIrqchip::new();
static EMMC: MassMediaController = unsafe { MassMediaController::new(EMMC_BASE) };
static UART: Pl011 = unsafe { Pl011::new(UART_BASE, UART_IRQ) };
pub(self) static BCM_MBOX: Bcm283xMailbox = unsafe { Bcm283xMailbox::new(BCM_MBOX_BASE) };
static LOCAL_TIMER: GenericTimer = GenericTimer::new(LOCAL_TIMER_IRQ);
