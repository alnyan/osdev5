use crate::arch::machine::{self, Bcm283xMailbox};
use crate::dev::sd::{
    SdCardIdentification, SdCardStatus, SdCommand, SdCommandNumber, SdCommandTransfer,
    SdHostController, SdResponse, SdResponseType,
};
use crate::dev::Device;
use crate::mem::virt::DeviceMemoryIo;
use crate::sync::IrqSafeSpinLock;
use crate::util::InitOnce;
use error::Errno;
use vfs::BlockDevice;

use tock_registers::interfaces::{Readable, Writeable};
use tock_registers::register_structs;
use tock_registers::registers::{ReadOnly, ReadWrite};

register_structs! {
    #[allow(non_snake_case)]
    Regs {
        (0x00 => ARG2: ReadWrite<u32>),
        (0x04 => BLKSIZECNT: ReadWrite<u32>),
        (0x08 => ARG1: ReadWrite<u32>),
        (0x0C => CMDTM: ReadWrite<u32>),
        (0x10 => RESP0: ReadOnly<u32>),
        (0x14 => RESP1: ReadOnly<u32>),
        (0x18 => RESP2: ReadOnly<u32>),
        (0x1C => RESP3: ReadOnly<u32>),
        (0x20 => DATA: ReadWrite<u32>),
        (0x24 => STATUS: ReadOnly<u32>),
        (0x28 => CONTROL0: ReadWrite<u32>),
        (0x2C => CONTROL1: ReadWrite<u32>),
        (0x30 => INTERRUPT: ReadWrite<u32>),
        (0x34 => IRPT_MASK: ReadWrite<u32>),
        (0x38 => IRPT_EN: ReadWrite<u32>),
        (0x3C => CONTROL2: ReadWrite<u32>),
        (0x40 => _res0),
        (0x50 => FORCE_IRPT: ReadWrite<u32>),
        (0x54 => _res1),
        (0x70 => BOOT_TIMEOUT: ReadWrite<u32>),
        (0x74 => DBG_SEL: ReadWrite<u32>),
        (0x78 => @END),
    }
}

struct MmcInner {
    regs: DeviceMemoryIo<Regs>,
    status: SdCardStatus,
}

pub struct MassMediaController {
    inner: InitOnce<IrqSafeSpinLock<MmcInner>>,
    base: usize,
}

fn clock_divider(f_base: u32, f_target: u32) -> Result<u32, Errno> {
    let mut target_div;
    let mut div = 0;
    if f_target <= f_base {
        target_div = f_base / f_target;
        if f_base % f_target != 0 {
            target_div -= 1;
        }
    } else {
        target_div = 1;
    }
    for first_bit in (0..31).rev() {
        if target_div & (1 << first_bit) != 0 {
            div = first_bit;
            target_div &= !(1 << first_bit);
            if target_div != 0 {
                div += 1;
            }
            break;
        }
    }

    if div == u32::MAX {
        div = 31;
    }
    if div >= 32 {
        div = 31;
    }
    if div != 0 {
        div = 1 << (div - 1)
    }
    if div >= 0x400 {
        div = 0x3FF;
    }

    let f_sel = div & 0xFF;
    let upper_bits = (div >> 8) & 0x3;

    Ok((f_sel << 8) | (upper_bits << 6))
}

impl MmcInner {
    fn power_on(&mut self) -> Result<(), Errno> {
        machine::BCM_MBOX.set_power_state(
            Bcm283xMailbox::POWER_SD_CARD,
            Bcm283xMailbox::POWER_STATE_ON | Bcm283xMailbox::POWER_STATE_WAIT,
        )
    }

    fn base_clock(&mut self) -> Result<u32, Errno> {
        machine::BCM_MBOX.clock_rate(Bcm283xMailbox::CLOCK_EMMC)
    }

    fn send_cmd_inner(&mut self, cmd: &mut SdCommand) -> Result<SdResponse, Errno> {
        debugln!("send_cmd {:?}", cmd.number);

        while self.regs.STATUS.get() & 1 != 0 {
            cortex_a::asm::nop();
        }

        let response_type = cmd.response_type();
        if response_type.is_busy() {
            // TODO check if this is an ABORT command

            while self.regs.STATUS.get() & 2 != 0 {
                cortex_a::asm::nop();
            }
        }

        let (block_count, block_size, io_flags) = match &cmd.transfer {
            SdCommandTransfer::Write(buf, blk_size) => {
                let sz = *blk_size as usize;
                assert!(buf.len() % sz == 0);
                ((buf.len() / sz) as u32, *blk_size, (1 << 21))
            }
            SdCommandTransfer::Read(buf, blk_size) => {
                let sz = *blk_size as usize;
                assert!(buf.len() % sz == 0);
                ((buf.len() / sz) as u32, *blk_size, (1 << 21) | (1 << 4))
            }
            SdCommandTransfer::None => (0, 512, 0),
        };

        let (size_136, size_flags) = match response_type {
            SdResponseType::R2 | SdResponseType::R4 => (true, 1 << 16),
            SdResponseType::R1b | SdResponseType::R5b => (false, 3 << 16),
            SdResponseType::None => (false, 0),
            _ => (false, 2 << 16),
        };

        debugln!("size_flags: {:#x}, io_flags: {:#x}", size_flags, io_flags);
        self.regs
            .BLKSIZECNT
            .set((block_size as u32) | (block_count << 16));
        self.regs.ARG1.set(cmd.argument);
        self.regs
            .CMDTM
            .set((cmd.number() << 24) | size_flags | io_flags);

        while self.regs.INTERRUPT.get() & 0x8001 == 0 {
            cortex_a::asm::nop();
        }

        let irq_status = self.regs.INTERRUPT.get();
        self.regs.INTERRUPT.set(0xFFFF0001);

        if irq_status & 0xFFFF0001 != 1 {
            warnln!("SD error: irq_status={:#x}", irq_status);
            return Err(Errno::InvalidArgument);
        }

        let response = if size_136 {
            SdResponse::Four([
                self.regs.RESP0.get(),
                self.regs.RESP1.get(),
                self.regs.RESP2.get(),
                self.regs.RESP3.get(),
            ])
        } else {
            SdResponse::One(self.regs.RESP0.get())
        };

        match &mut cmd.transfer {
            SdCommandTransfer::Write(_, _) => {
                todo!()
            }
            SdCommandTransfer::Read(buf, _) => {
                debugln!("Reading {} data blocks", block_count);
                for i in 0..block_count {
                    while self.regs.INTERRUPT.get() & (0x8000 | (1 << 5)) == 0 {
                        cortex_a::asm::nop();
                    }
                    let irq_status = self.regs.INTERRUPT.get();
                    self.regs.INTERRUPT.set(0xFFFF0000 | (1 << 5));

                    if irq_status & (0xFFFF0000 | (1 << 5)) != (1 << 5) {
                        todo!();
                    }

                    assert!(block_size % 4 == 0);
                    for j in (0..block_size).step_by(4) {
                        let word = self.regs.DATA.get();
                        let base = (i * block_size as u32) as usize + j as usize;
                        buf[base + 0] = (word & 0xFF) as u8;
                        buf[base + 1] = ((word >> 8) & 0xFF) as u8;
                        buf[base + 2] = ((word >> 16) & 0xFF) as u8;
                        buf[base + 3] = (word >> 24) as u8;
                    }
                }
            }
            SdCommandTransfer::None => {}
        }

        Ok(response)
    }

    fn phys_reset(&mut self) -> Result<(), Errno> {
        self.status.phys_inserted = false;

        let mut tmp = self.regs.CONTROL1.get();
        tmp |= 1 << 24;
        // Disable clock
        tmp &= !(1 << 2);
        tmp &= !(1 << 0);
        self.regs.CONTROL1.set(tmp);

        debugln!("Checking for a card");
        let mut retry = 5;
        let mut status = 0;

        while retry > 0 {
            status = self.regs.STATUS.get();
            if status & 1 << 16 != 0 {
                break;
            }

            for _ in 0..100000 {
                cortex_a::asm::nop();
            }

            retry -= 1;
        }

        if retry == 0 {
            warnln!("No card inserted");
            return Ok(());
        }

        self.status.phys_inserted = true;
        debugln!("A card is present: status={:#x}", status);

        self.regs.CONTROL2.set(0);

        let mut f_base = self.base_clock()?;
        if f_base == 0 {
            f_base = 100000000;
        }

        let div = clock_divider(f_base, 400000)?;

        debugln!("Switching to ID frequency");

        tmp = self.regs.CONTROL1.get();
        tmp |= div;
        tmp &= !(0xF << 16);
        tmp |= 11 << 16;
        tmp |= 4;
        tmp |= 1;
        self.regs.CONTROL1.set(tmp);

        while self.regs.CONTROL1.get() & 1 << 1 == 0 {
            cortex_a::asm::nop();
        }

        self.regs.IRPT_EN.set(0);
        self.regs.INTERRUPT.set(u32::MAX);
        self.regs.IRPT_MASK.set(u32::MAX);

        Ok(())
    }

    fn send_cmd(&mut self, cmd: &mut SdCommand) -> Result<SdResponse, Errno> {
        if cmd.is_acmd() {
            let arg = if let Some(rca) = self.status.address {
                (rca as u32) << 16
            } else {
                0
            };
            self.send_cmd_inner(&mut SdCommand {
                number: SdCommandNumber::Cmd55,
                argument: arg,
                transfer: SdCommandTransfer::None,
            })?;
        }
        self.send_cmd_inner(cmd)
    }
}

impl BlockDevice for MassMediaController {
    fn read(&self, pos: usize, data: &mut [u8]) -> Result<(), Errno> {
        // TODO check card status
        if data.len() % 512 != 0 || pos % 512 != 0 {
            todo!()
        }

        for i in 0..(data.len() / 512) {
            let s = i * 512;
            self.send_cmd(&mut SdCommand {
                number: SdCommandNumber::Cmd17,
                argument: (pos / 512 + i) as u32,
                transfer: SdCommandTransfer::Read(&mut data[s..(s + 512)], 512),
            })?;
        }
        Ok(())
    }

    fn write(&self, _pos: usize, _data: &[u8]) -> Result<(), Errno> {
        todo!()
    }
}

impl SdHostController for MassMediaController {
    fn send_cmd(&self, cmd: &mut SdCommand) -> Result<SdResponse, Errno> {
        self.inner.get().lock().send_cmd(cmd)
    }

    fn phys_reset(&self) -> Result<(), Errno> {
        let mut inner = self.inner.get().lock();
        inner.status.address = None;
        inner.status.id = None;
        inner.phys_reset()
    }

    fn is_phys_inserted(&self) -> bool {
        self.inner.get().lock().status.phys_inserted
    }

    fn set_transfer_block_size(&self, blk: usize) -> Result<(), Errno> {
        // TODO other block sizes?
        assert_eq!(blk, 512);
        let inner = self.inner.get().lock();
        let mut tmp = inner.regs.BLKSIZECNT.get();
        tmp &= !0xFFF;
        tmp |= 0x200;
        inner.regs.BLKSIZECNT.set(tmp);
        Ok(())
    }

    fn set_card_address(&self, rca: u16) -> Result<(), Errno> {
        self.inner.get().lock().status.address = Some(rca);
        Ok(())
    }

    fn set_card_identification(&self, id: SdCardIdentification) -> Result<(), Errno> {
        self.inner.get().lock().status.id = Some(id);
        Ok(())
    }

    fn reset_card_identification(&self) -> Result<(), Errno> {
        self.inner.get().lock().status.id = None;
        Ok(())
    }
}

impl Device for MassMediaController {
    fn name(&self) -> &'static str {
        "BCM283x External Mass Media Controller"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        let mut inner = MmcInner {
            regs: DeviceMemoryIo::map(self.name(), self.base, 1)?,
            status: SdCardStatus::invalid(),
        };
        inner.power_on()?;
        self.inner.init(IrqSafeSpinLock::new(inner));

        self.reset_card()?;
        Ok(())
    }
}

impl MassMediaController {
    pub const unsafe fn new(base: usize) -> Self {
        Self {
            inner: InitOnce::new(),
            base,
        }
    }
}
