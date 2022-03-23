use crate::arch::x86_64::{
    idt::{Entry as IdtEntry, SIZE as IDT_SIZE},
    PortIo,
};
use crate::dev::{
    irq::{IntController, IntSource, IrqContext},
    Device,
};
use crate::sync::IrqSafeSpinLock;
use core::arch::global_asm;
use libsys::error::Errno;

const ICW1_INIT: u8 = 0x10;
const ICW1_ICW4: u8 = 0x01;

const ICW4_8086: u8 = 0x01;

pub(super) struct I8259 {
    cmd_a: PortIo<u8>,
    cmd_b: PortIo<u8>,
    data_a: PortIo<u8>,
    data_b: PortIo<u8>,

    table: IrqSafeSpinLock<[Option<&'static (dyn IntSource + Sync)>; 15]>,
}

/// Interrupt line number wrapper struct
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct IrqNumber(u32);

impl IrqNumber {
    /// IRQ line number limit
    pub const MAX: u32 = 16;

    /// Constructs a wrapped IRQ line number
    pub const fn new(u: u32) -> Self {
        if u > Self::MAX {
            panic!();
        }
        Self(u)
    }
}

impl Device for I8259 {
    fn name(&self) -> &'static str {
        "i8259-compatible IRQ controller"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        self.cmd_a.write(ICW1_INIT | ICW1_ICW4);
        self.cmd_b.write(ICW1_INIT | ICW1_ICW4);
        self.data_a.write(32);
        self.data_b.write(32 + 8);
        self.data_a.write(4);
        self.data_b.write(2);

        self.data_a.write(ICW4_8086);
        self.data_b.write(ICW4_8086);

        self.data_a.write(0xFE);
        self.data_b.write(0xFF);

        Ok(())
    }
}

impl IntController for I8259 {
    type IrqNumber = IrqNumber;

    fn register_handler(
        &self,
        irq: Self::IrqNumber,
        handler: &'static (dyn IntSource + Sync),
    ) -> Result<(), Errno> {
        let index = irq.0 as usize;
        let mut lock = self.table.lock();
        if lock[index].is_some() {
            return Err(Errno::AlreadyExists);
        }

        lock[index] = Some(handler);
        Ok(())
    }

    fn enable_irq(&self, irq: Self::IrqNumber) -> Result<(), Errno> {
        let port = if irq.0 < 8 {
            &self.data_a
        } else {
            &self.data_b
        };

        let mask = port.read() & !(1 << (irq.0 & 0x7));
        port.write(mask);
        Ok(())
    }

    fn handle_pending_irqs<'irq_context>(&'irq_context self, ic: &IrqContext<'irq_context>) {
        let irq_number = ic.token();

        // Clear irq
        if irq_number > 8 {
            self.cmd_b.write(0x20);
        }
        self.cmd_a.write(0x20);

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

impl I8259 {
    pub const fn new() -> Self {
        unsafe {
            Self {
                cmd_a: PortIo::new(0x20),
                data_a: PortIo::new(0x21),
                cmd_b: PortIo::new(0xA0),
                data_b: PortIo::new(0xA1),
                table: IrqSafeSpinLock::new([None; 15]),
            }
        }
    }
}

pub fn map_isr_entries(entries: &mut [IdtEntry; IDT_SIZE]) {
    extern "C" {
        static __x86_64_irq_vectors: [usize; 16];
    }

    for (i, &entry) in unsafe { __x86_64_irq_vectors.iter().enumerate() } {
        entries[i + 32] = IdtEntry::new(entry, 0x08, IdtEntry::PRESENT | IdtEntry::INT32);
    }
}

global_asm!(include_str!("irq_vectors.S"), options(att_syntax));
