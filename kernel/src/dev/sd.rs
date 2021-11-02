use crate::dev::Device;
use error::Errno;
use vfs::BlockDevice;

pub trait SdHostController: Device + BlockDevice {
    // Physical layer
    fn send_cmd(&self, cmd: &mut SdCommand) -> Result<SdResponse, Errno>;
    fn phys_reset(&self) -> Result<(), Errno>;
    fn is_phys_inserted(&self) -> bool;

    // Data layer
    fn set_card_address(&self, rca: u16) -> Result<(), Errno>;
    fn set_card_identification(&self, id: SdCardIdentification) -> Result<(), Errno>;
    fn reset_card_identification(&self) -> Result<(), Errno>;

    fn reset_card_probe(&self) -> Result<(), Errno> {
        self.send_cmd(&mut SdCommand {
            number: SdCommandNumber::Cmd0,
            argument: 0,
            transfer: SdCommandTransfer::None,
        })?;

        if self
            .send_cmd(&mut SdCommand {
                number: SdCommandNumber::Cmd8,
                argument: 0x1AA,
                transfer: SdCommandTransfer::None,
            })?
            .unwrap_one()
            != 0x1AA
        {
            warnln!("Card did not respond to CMD8");
            return Err(Errno::DeviceError);
        }

        // Set operating conditions
        for _ in 0..10 {
            let res = self
                .send_cmd(&mut SdCommand {
                    number: SdCommandNumber::Acmd41,
                    argument: 0x00FF8000,
                    transfer: SdCommandTransfer::None,
                })?
                .unwrap_one();

            if res & (1 << 31) != 0 {
                return Ok(());
            }

            for _ in 0..1000000 {
                cortex_a::asm::nop();
            }
        }

        warnln!("Card did not respond to Acmd41");
        Err(Errno::DeviceError)
    }

    fn reset_card(&self) -> Result<(), Errno> {
        let mut buf = [0u8; 16];

        self.reset_card_identification()?;

        // Reset physical interface
        self.phys_reset()?;

        // Bail out if card is not physically present
        if !self.is_phys_inserted() {
            return Ok(());
        }

        // Probe for card
        self.reset_card_probe()?;

        // Perform init sequence
        let cmd2 = self
            .send_cmd(&mut SdCommand {
                number: SdCommandNumber::Cmd2,
                argument: 0,
                transfer: SdCommandTransfer::None,
            })?
            .unwrap_four();

        let cmd3 = self
            .send_cmd(&mut SdCommand {
                number: SdCommandNumber::Cmd3,
                argument: 0,
                transfer: SdCommandTransfer::None,
            })?
            .unwrap_one();

        let rca = (cmd3 >> 16) as u16;

        if cmd3 & (1 << 14) != 0 {
            warnln!("Illegal command");
            return Err(Errno::DeviceError);
        }

        if cmd3 & (1 << 13) != 0 {
            warnln!("Card reported error");
            return Err(Errno::DeviceError);
        }

        if cmd3 & (1 << 8) == 0 {
            warnln!("Card is not ready for data mode");
            return Err(Errno::DeviceError);
        }
        let cmd9 = self
            .send_cmd(&mut SdCommand {
                number: SdCommandNumber::Cmd9,
                argument: cmd3 & 0xFFFF0000,
                transfer: SdCommandTransfer::None,
            })?
            .unwrap_four();

        debugln!("cmd9 = {:#x?}", cmd9);
        let csd_structure = (cmd9[3] >> 16) & 0x3;
        let capacity = match csd_structure {
            0 => {
                let c_size = ((cmd9[2] & 0x3) << 10) | ((cmd9[1] >> 22) & 0x3FF);
                let c_size_mult = (cmd9[1] >> 7) & 0x7;
                ((c_size + 1) as u64) << (c_size_mult + 9 /* Block size is 512 */ + 2)
            }
            1 => todo!(),
            _ => {
                warnln!("Invalid CSD version: {}", csd_structure);
                return Err(Errno::DeviceError);
            }
        };

        let cmd7 = self
            .send_cmd(&mut SdCommand {
                number: SdCommandNumber::Cmd7,
                argument: cmd3 & 0xFFFF0000,
                transfer: SdCommandTransfer::None,
            })?
            .unwrap_one();

        let status = (cmd7 >> 9) & 0xF;
        if status != 3 && status != 4 {
            warnln!("Card reported invalid status: {}", status);
            return Err(Errno::DeviceError);
        }

        // Set block size
        self.send_cmd(&mut SdCommand {
            number: SdCommandNumber::Cmd16,
            argument: 512,
            transfer: SdCommandTransfer::None,
        })?;

        self.set_card_address(rca)?;
        self.send_cmd(&mut SdCommand {
            number: SdCommandNumber::Acmd51,
            argument: 0,
            transfer: SdCommandTransfer::Read(&mut buf[..8], 8),
        })?;

        let sd_spec = buf[0] & 0xF;

        let version = match sd_spec {
            0 => SdCardVersion::Ver10,
            1 => SdCardVersion::Ver11,
            2 => {
                // FIXME check for 3.0/4.0
                SdCardVersion::Ver20
            }
            _ => panic!("Invalid version: {:#x}", sd_spec),
        };

        let card_id = SdCardIdentification {
            id: cmd2,
            version,
            capacity,
        };

        self.set_card_identification(card_id)?;
        infoln!("Found a valid SD card of capacity {}B", capacity);

        // TODO High speed support
        // if version >= SdCardVersion::Ver11 {
        //     let mut buf = [0u8; 64];
        //     self.send_cmd(&mut SdCommand {
        //         number: SdCommandNumber::Cmd6,
        //         argument: 0x00FFFFF0,
        //         transfer: SdCommandTransfer::Read(&mut buf, 64)
        //     })?;

        //     // Check HS support
        //     let hs_support = buf[13] >> 1 != 0;
        //     if hs_support {
        //         debugln!("Switching to high speed mode");
        //         self.send_cmd(&mut SdCommand {
        //             number: SdCommandNumber::Cmd6,
        //             argument: 0x80FFFFF1,
        //             transfer: SdCommandTransfer::None
        //         })?;

        //         todo!();
        //     }
        // }

        // let dbus_widths = buf[0] & 0xF;
        // TODO 4 bit mode support

        // TODO switch clock rate

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SdResponseType {
    None,
    R1,
    R1b,
    R2,
    R3,
    R4,
    R5,
    R5b,
    R6,
    R7,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum SdCardVersion {
    Ver10 = 0x10,
    Ver11 = 0x11,
    Ver20 = 0x20,
    Ver30 = 0x30,
    Ver40 = 0x40,
}

pub struct SdCardIdentification {
    pub id: [u32; 4],
    pub version: SdCardVersion,
    pub capacity: u64,
}

pub struct SdCardStatus {
    pub phys_inserted: bool,
    pub address: Option<u16>,
    pub id: Option<SdCardIdentification>,
}

#[derive(Clone, Copy, Debug)]
pub enum SdResponse {
    One(u32),
    Four([u32; 4]),
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u32)]
pub enum SdCommandNumber {
    Cmd0 = 0,
    Cmd2 = 2,
    Cmd3 = 3,
    Cmd6 = 6,
    Cmd7 = 7,
    Cmd8 = 8,
    Cmd9 = 9,
    Cmd16 = 16,
    Cmd17 = 17,
    Acmd41 = 41,
    Acmd51 = 51,
    Cmd55 = 55,
}

pub struct SdCommandInfo {
    pub response_type: SdResponseType,
}

pub enum SdCommandTransfer<'a> {
    None,
    Read(&'a mut [u8], u16),
    Write(&'a [u8], u16),
}

pub struct SdCommand<'a> {
    pub number: SdCommandNumber,
    pub argument: u32,
    pub transfer: SdCommandTransfer<'a>,
}

impl SdCardStatus {
    pub const fn invalid() -> Self {
        Self {
            phys_inserted: false,
            address: None,
            id: None,
        }
    }
}

impl SdResponseType {
    pub const fn is_busy(self) -> bool {
        match self {
            Self::R1b | Self::R5b => true,
            _ => false,
        }
    }
}

impl SdResponse {
    pub fn unwrap_one(&self) -> u32 {
        match self {
            &SdResponse::One(v) => v,
            _ => panic!("Unexpected response type"),
        }
    }

    pub fn unwrap_four(&self) -> [u32; 4] {
        match self {
            &SdResponse::Four(v) => v,
            _ => panic!("Unexpected response type"),
        }
    }
}

impl SdCommandInfo {
    pub const fn new(response_type: SdResponseType) -> Self {
        Self { response_type }
    }
}

impl SdCommand<'_> {
    pub const fn info_struct(&self) -> SdCommandInfo {
        match self.number {
            SdCommandNumber::Cmd0 => SdCommandInfo::new(SdResponseType::None),
            SdCommandNumber::Cmd2 => SdCommandInfo::new(SdResponseType::R2),
            SdCommandNumber::Cmd3 => SdCommandInfo::new(SdResponseType::R6),
            SdCommandNumber::Cmd6 => SdCommandInfo::new(SdResponseType::R1),
            SdCommandNumber::Cmd7 => SdCommandInfo::new(SdResponseType::R1b),
            SdCommandNumber::Cmd8 => SdCommandInfo::new(SdResponseType::R7),
            SdCommandNumber::Cmd9 => SdCommandInfo::new(SdResponseType::R2),
            SdCommandNumber::Cmd16 => SdCommandInfo::new(SdResponseType::R1),
            SdCommandNumber::Cmd17 => SdCommandInfo::new(SdResponseType::R1),
            SdCommandNumber::Acmd41 => SdCommandInfo::new(SdResponseType::R3),
            SdCommandNumber::Acmd51 => SdCommandInfo::new(SdResponseType::R1),
            SdCommandNumber::Cmd55 => SdCommandInfo::new(SdResponseType::R1),
        }
    }

    pub const fn is_acmd(&self) -> bool {
        match self.number {
            SdCommandNumber::Acmd41 | SdCommandNumber::Acmd51 => true,
            _ => false,
        }
    }

    pub const fn number(&self) -> u32 {
        self.number as u32
    }
}
