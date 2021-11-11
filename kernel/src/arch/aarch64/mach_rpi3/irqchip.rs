use crate::dev::{
    irq::{IntController, IntSource, IrqContext},
    Device,
};
use crate::mem::virt::DeviceMemoryIo;
use crate::sync::IrqSafeSpinLock;
use crate::util::InitOnce;
use core::fmt;
use cortex_a::registers::MPIDR_EL1;
use libsys::error::Errno;

use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::register_structs;
use tock_registers::registers::{ReadOnly, ReadWrite};

register_structs! {
    #[allow(non_snake_case)]
    pub(super) BcmRegs {
        (0x000 => _res0),
        (0x200 => PENDING_BASIC: ReadOnly<u32>),
        (0x204 => PENDING1: ReadOnly<u32>),
        (0x208 => PENDING2: ReadOnly<u32>),
        (0x20C => _res1),
        (0x210 => ENABLE1: ReadWrite<u32>),
        (0x214 => ENABLE2: ReadWrite<u32>),
        (0x218 => ENABLE_BASIC: ReadWrite<u32>),
        (0x21C => @END),
    }
}

register_structs! {
    #[allow(non_snake_case)]
    pub(super) Qa7Regs {
        (0x000 => _res0),
        (0x040 => CORE_IRQ_EN: [ReadWrite<u32>; 4]),
        (0x050 => _res1),
        (0x060 => CORE_IRQ_SRC: [ReadWrite<u32>; 4]),
        (0x070 => @END),
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct IrqNumber(u32);

impl IrqNumber {
    pub const MAX: u32 = 64 + 32;

    pub const fn bcm_irq(n: u32) -> Self {
        assert!(n < 64);
        Self(n + 32)
    }

    pub const fn qa7_irq(n: u32) -> Self {
        assert!(n < 32);
        Self(n)
    }

    pub const fn is_bcm_irq(self) -> bool {
        self.0 >= 32
    }

    pub const fn number(self) -> u32 {
        if self.is_bcm_irq() {
            self.0 - 32
        } else {
            self.0
        }
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Debug for IrqNumber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}_irq{}",
            if self.is_bcm_irq() { "bcm" } else { "qa7" },
            self.number()
        )
    }
}

struct BcmIrqchipInner {
    regs: DeviceMemoryIo<BcmRegs>,
}
struct Qa7IrqchipInner {
    regs: DeviceMemoryIo<Qa7Regs>,
}

pub struct Bcm283xIrqchip {
    bcm_inner: InitOnce<IrqSafeSpinLock<BcmIrqchipInner>>,
    qa7_inner: InitOnce<IrqSafeSpinLock<Qa7IrqchipInner>>,
    table: IrqSafeSpinLock<[Option<&'static (dyn IntSource + Sync)>; IrqNumber::MAX as usize]>,
}

impl BcmIrqchipInner {
    fn enable_irq(&self, n: u32) -> Result<(), Errno> {
        let (reg, bit) = if n < 32 {
            (&self.regs.ENABLE1, 1 << n)
        } else if n < 64 {
            (&self.regs.ENABLE2, 1 << (n - 32))
        } else {
            todo!();
        };
        reg.set(reg.get() | bit);
        Ok(())
    }

    fn pending_irq(&self) -> Option<u32> {
        let status = self.regs.PENDING2.get();
        for bit in 0..32 {
            if status & (1 << bit) != 0 {
                return Some(bit + 32);
            }
        }
        None
    }
}

impl Qa7IrqchipInner {
    fn enable_irq(&self, n: u32) -> Result<(), Errno> {
        // TODO check this code in SMP setup
        let core_id = MPIDR_EL1.get() & 0x3;
        // Timer IRQ control
        let (reg, bit) = if n < 4 {
            (&self.regs.CORE_IRQ_EN[core_id as usize], 1 << n)
        } else {
            todo!()
        };
        reg.set(reg.get() | bit);
        Ok(())
    }

    fn pending_irq(&self) -> Option<u32> {
        let core_id = MPIDR_EL1.get() & 0x3;
        let reg = &self.regs.CORE_IRQ_SRC[core_id as usize];
        let value = reg.get();
        for bit in 0..8 {
            if value & (1 << bit) != 0 {
                return Some(bit);
            }
        }
        None
    }

    fn clear_irq(&self) {
        let core_id = MPIDR_EL1.get() & 0x3;
        let reg = &self.regs.CORE_IRQ_SRC[core_id as usize];
        reg.set(0);
    }
}

impl Device for Bcm283xIrqchip {
    fn name(&self) -> &'static str {
        "BCM283x/QA7 Interrupt Controller"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        self.bcm_inner.init(IrqSafeSpinLock::new(BcmIrqchipInner {
            regs: DeviceMemoryIo::map("BCM283x Peripheral Interrupt Controller", 0x3F00B000, 1)?,
        }));
        self.qa7_inner.init(IrqSafeSpinLock::new(Qa7IrqchipInner {
            regs: DeviceMemoryIo::map("QA7 Core Interrupt Controller", 0x40000000, 1)?,
        }));
        Ok(())
    }
}

impl IntController for Bcm283xIrqchip {
    type IrqNumber = IrqNumber;

    fn register_handler(
        &self,
        irq: IrqNumber,
        handler: &'static (dyn IntSource + Sync),
    ) -> Result<(), Errno> {
        let mut table = self.table.lock();
        let irqi = irq.index();
        if table[irqi as usize].is_some() {
            return Err(Errno::AlreadyExists);
        }

        debugln!("Bound {:?} to {:?}", irq, Device::name(handler));
        table[irqi as usize] = Some(handler);

        Ok(())
    }

    fn enable_irq(&self, irq: IrqNumber) -> Result<(), Errno> {
        if irq.is_bcm_irq() {
            self.bcm_inner.get().lock().enable_irq(irq.number())
        } else {
            self.qa7_inner.get().lock().enable_irq(irq.number())
        }
    }

    fn handle_pending_irqs<'q>(&'q self, _ic: &IrqContext<'q>) {
        let qa7 = self.qa7_inner.get().lock();
        let bcm = self.bcm_inner.get().lock();

        let irq_number = if let Some(irq) = qa7.pending_irq() {
            irq as usize
        } else if let Some(irq) = bcm.pending_irq() {
            irq as usize + 32
        } else {
            panic!("No IRQ pending");
        };

        drop(bcm);

        if irq_number < 32 {
            qa7.clear_irq();
            drop(qa7);
        }

        {
            let table = self.table.lock();
            match table[irq_number] {
                None => panic!("No handler registered for irq{}", irq_number),
                Some(handler) => {
                    drop(table);
                    handler.handle_irq().expect("irq handler failed")
                }
            }
        }
    }
}

impl Bcm283xIrqchip {
    pub const fn new() -> Self {
        Self {
            bcm_inner: InitOnce::new(),
            qa7_inner: InitOnce::new(),
            table: IrqSafeSpinLock::new([None; IrqNumber::MAX as usize]),
        }
    }
}
