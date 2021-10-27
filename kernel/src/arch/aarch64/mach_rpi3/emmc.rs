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
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};
use tock_registers::registers::{ReadOnly, ReadWrite};
use tock_registers::{register_bitfields, register_structs};
use vfs::BlockDevice;

register_bitfields! {
    u32,
    BLKSIZECNT [
        BLKCNT OFFSET(16) NUMBITS(16) [],
        BLKSIZE OFFSET(0) NUMBITS(10) [],
    ],
    CMDTM [
        CMD_INDEX OFFSET(24) NUMBITS(6) [],
        CMD_TYPE OFFSET(22) NUMBITS(23) [
            Normal = 0,
            Suspend = 1,
            Resume = 2,
            Abort = 3,
        ],
        CMD_ISDATA OFFSET(21) NUMBITS(1) [],
        CMD_IXCHK_EN OFFSET(20) NUMBITS(1) [],
        CMD_CRCCHK_EN OFFSET(19) NUMBITS(1) [],
        CMD_RSPNS_TYPE OFFSET(16) NUMBITS(2) [
            None = 0,
            Bits136 = 1,
            Bits48 = 2,
            Bits48Busy = 3
        ],
        TM_MULTI_BLOCK OFFSET(5) NUMBITS(1) [],
        TM_DAT_DIR OFFSET(4) NUMBITS(1) [
            HostToCard = 0,
            CardToHost = 1,
        ],
        TM_AUDO_CMD_EN OFFSET(2) NUMBITS(2) [
            None = 0,
            Cmd12 = 1,
            Cmd23 = 2
        ],
        TM_BLKCNT_EN OFFSET(1) NUMBITS(1) [],
    ],
    STATUS [
        READ_TRANSFER OFFSET(9) NUMBITS(1) [],
        WRITE_TRANSFER OFFSET(8) NUMBITS(1) [],
        MISC_INSERTED OFFSET(16) NUMBITS(1) [],
        DAT_ACTIVE OFFSET(2) NUMBITS(1) [],
        DAT_INHIBIT OFFSET(1) NUMBITS(1) [],
        CMD_INHIBIT OFFSET(0) NUMBITS(1) [],
    ],
    INTERRUPT [
        ACMD_ERR 24,
        DEND_ERR 22,
        DCRC_ERR 21,
        DTO_ERR 20,
        CBAD_ERR 19,
        CEND_ERR 18,
        CCRC_ERR 17,
        CTO_ERR 16,
        ERR 15,
        ENDBOOT 14,
        BOOTACK 13,
        RETUNE 12,
        CARD 8,
        READ_RDY 5,
        WRITE_RDY 4,
        BLOCK_GAP 2,
        DATA_DONE 1,
        CMD_DONE 0,
    ],
    CONTROL1 [
        SRST_DATA OFFSET(26) NUMBITS(1) [],
        SRST_CMD OFFSET(25) NUMBITS(1) [],
        SRST_HC OFFSET(24) NUMBITS(1) [],
        DATA_TOUNIT OFFSET(16) NUMBITS(4) [],
        CLK_FREQ8 OFFSET(8) NUMBITS(8) [],
        CLK_FREQ_MS2 OFFSET(6) NUMBITS(2) [],
        CLK_GENSEL OFFSET(5) NUMBITS(1) [
            Divided = 0,
            Programmable = 1
        ],
        CLK_EN OFFSET(2) NUMBITS(1) [],
        CLK_STABLE OFFSET(1) NUMBITS(1) [],
        CLK_INTLEN OFFSET(0) NUMBITS(1) [],
    ]
}

register_structs! {
    #[allow(non_snake_case)]
    Regs {
        (0x00 => ARG2: ReadWrite<u32>),
        (0x04 => BLKSIZECNT: ReadWrite<u32, BLKSIZECNT::Register>),
        (0x08 => ARG1: ReadWrite<u32>),
        (0x0C => CMDTM: ReadWrite<u32, CMDTM::Register>),
        (0x10 => RESP0: ReadOnly<u32>),
        (0x14 => RESP1: ReadOnly<u32>),
        (0x18 => RESP2: ReadOnly<u32>),
        (0x1C => RESP3: ReadOnly<u32>),
        (0x20 => DATA: ReadWrite<u32>),
        (0x24 => STATUS: ReadOnly<u32, STATUS::Register>),
        (0x28 => CONTROL0: ReadWrite<u32>),
        (0x2C => CONTROL1: ReadWrite<u32, CONTROL1::Register>),
        (0x30 => INTERRUPT: ReadWrite<u32, INTERRUPT::Register>),
        (0x34 => IRPT_MASK: ReadWrite<u32, INTERRUPT::Register>),
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

fn clock_divider(f_base: u32, f_target: u32) -> u32 {
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
        div = 1 << (div - 1);
    }
    if div >= 0x400 {
        div = 0x3FF;
    }

    div
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

    // TODO generalize flag setting
    fn send_cmd_inner(&mut self, cmd: &mut SdCommand) -> Result<SdResponse, Errno> {
        const TIMEOUT: u64 = 10000;
        let info = cmd.info_struct();
        let mut cmdtm = CMDTM::CMD_INDEX.val(cmd.number as u32);

        // Wait until CMD lines free up
        crate::block!(
            self.regs.STATUS.matches_all(STATUS::CMD_INHIBIT::CLEAR),
            TIMEOUT
        );

        if info.response_type.is_busy() {
            // TODO check if this is an ABORT command
            // Wait until DAT lines free up after busy cmd
            crate::block!(
                self.regs.STATUS.matches_all(STATUS::DAT_INHIBIT::CLEAR),
                TIMEOUT
            );
        }

        let (block_count, block_size) = match &cmd.transfer {
            SdCommandTransfer::Write(buf, blk_size) => {
                let sz = *blk_size as usize;
                assert!(buf.len() % sz == 0);
                cmdtm += CMDTM::CMD_ISDATA::SET + CMDTM::TM_DAT_DIR::HostToCard;
                ((buf.len() / sz) as u32, *blk_size)
            }
            SdCommandTransfer::Read(buf, blk_size) => {
                let sz = *blk_size as usize;
                assert!(buf.len() % sz == 0);
                cmdtm += CMDTM::CMD_ISDATA::SET + CMDTM::TM_DAT_DIR::CardToHost;
                ((buf.len() / sz) as u32, *blk_size)
            }
            SdCommandTransfer::None => (0, 512),
        };

        let size_136 = match info.response_type {
            SdResponseType::R2 | SdResponseType::R4 => {
                cmdtm += CMDTM::CMD_RSPNS_TYPE::Bits136;
                true
            }
            SdResponseType::R1b | SdResponseType::R5b => {
                cmdtm += CMDTM::CMD_RSPNS_TYPE::Bits48Busy;
                false
            }
            SdResponseType::None => false,
            _ => {
                cmdtm += CMDTM::CMD_RSPNS_TYPE::Bits48;
                false
            }
        };

        self.regs.BLKSIZECNT.write(
            BLKSIZECNT::BLKCNT.val(block_count) + BLKSIZECNT::BLKSIZE.val(block_size as u32),
        );
        self.regs.ARG1.set(cmd.argument);
        self.regs.CMDTM.write(cmdtm);
        crate::block!(
            self.regs
                .INTERRUPT
                .matches_any(INTERRUPT::ERR::SET + INTERRUPT::CMD_DONE::SET),
            10000
        );

        let irq_status = self.regs.INTERRUPT.get();
        self.regs.INTERRUPT.set(0xFFFF0001);

        if irq_status & 0xFFFF0000 != 0 {
            warnln!("SD error: irq_status={:#x}", irq_status);
            return Err(Errno::InvalidArgument);
        }
        if !INTERRUPT::CMD_DONE.is_set(irq_status) {
            warnln!("SD command did not report properly");
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
                    crate::block!(
                        self.regs
                            .INTERRUPT
                            .matches_any(INTERRUPT::ERR::SET + INTERRUPT::READ_RDY::SET),
                        10000
                    );
                    let irq_status = self.regs.INTERRUPT.get();
                    self.regs.INTERRUPT.set(0xFFFF0000 | (1 << 5));

                    if irq_status & 0xFFFF0000 != 0 {
                        warnln!("SD error during data read: irq_status={:#x}", irq_status);
                        return Err(Errno::InvalidArgument);
                    }
                    if !INTERRUPT::READ_RDY.is_set(irq_status) {
                        warnln!("SD did not respond with data blocks");
                        return Err(Errno::InvalidArgument);
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

        self.regs
            .CONTROL1
            .modify(CONTROL1::SRST_HC::SET + CONTROL1::CLK_EN::CLEAR + CONTROL1::CLK_INTLEN::CLEAR);

        // Wait for SRST_HC to clear
        crate::block!(
            self.regs.CONTROL1.matches_all(CONTROL1::SRST_HC::CLEAR),
            10000
        );

        //let mut tmp;

        debugln!("Checking for a card");
        crate::block!(
            self.regs.STATUS.matches_all(STATUS::MISC_INSERTED::SET),
            10000,
            {
                warnln!("No card inserted");
                return Ok(());
            }
        );

        self.status.phys_inserted = true;

        self.regs.CONTROL2.set(0);

        let mut f_base = self.base_clock()?;
        if f_base == 0 {
            f_base = 100000000;
        }

        let div = clock_divider(f_base, 400000);
        debugln!("Switching to ID frequency");

        let div_lsb = div & 0xFF;
        let div_msb = (div >> 8) & 0x3;
        self.regs.CONTROL1.modify(
            CONTROL1::CLK_FREQ8.val(div_lsb)
                + CONTROL1::CLK_FREQ_MS2.val(div_msb)
                + CONTROL1::DATA_TOUNIT.val(11)
                + CONTROL1::CLK_EN::SET
                + CONTROL1::CLK_INTLEN::SET,
        );

        crate::block!(
            self.regs.CONTROL1.matches_all(CONTROL1::CLK_STABLE::SET),
            100000,
            {
                warnln!("Controller clock did not stabilize in time");
                return Err(Errno::TimedOut);
            }
        );

        // Do not forward any IRQs to ARM side
        self.regs.IRPT_EN.set(0);
        // Ack and unmask all interrupts to the controller
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
