//! SD host controller interface and card operation facilities
use crate::dev::Device;
use error::Errno;
use vfs::BlockDevice;

/// Generic SD/MMC host controller interface
pub trait SdHostController: Device + BlockDevice {
    // Physical layer
    /// Sends `cmd` to the card using controller's physical layer
    fn send_cmd(&self, cmd: &mut SdCommand) -> Result<SdResponse, Errno>;
    /// Performs controller reset
    fn phys_reset(&self) -> Result<(), Errno>;
    /// Returns `true` if the card is physically present
    fn is_phys_inserted(&self) -> bool;

    // Data layer
    /// Sets card relative address
    fn set_card_address(&self, rca: u16) -> Result<(), Errno>;
    /// Sets card identification data
    fn set_card_identification(&self, id: SdCardIdentification) -> Result<(), Errno>;
    /// Resets card to unidentified state
    fn reset_card_identification(&self) -> Result<(), Errno>;

    /// Resets the inserted card and waits for it to respond
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

    /// Performs controller reset and attempts SD card initialization sequence
    /// if it is physically present
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

/// List of possible response types by SD cards
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SdResponseType {
    /// No response
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

/// List of possible SD card versions
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum SdCardVersion {
    Ver10 = 0x10,
    Ver11 = 0x11,
    Ver20 = 0x20,
    Ver30 = 0x30,
    Ver40 = 0x40,
}

/// SD card identification data
pub struct SdCardIdentification {
    /// Manufacturer's/device ID
    pub id: [u32; 4],
    /// SD card version
    pub version: SdCardVersion,
    /// SD card capacity in bytes
    pub capacity: u64,
}

/// SD card status data
pub struct SdCardStatus {
    /// If `true`, SD card is physically detected by the controller
    pub phys_inserted: bool,
    /// SD card's RCA (relative card address)
    pub address: Option<u16>,
    /// Identification data
    pub id: Option<SdCardIdentification>,
}

/// List of possible SD command responses
#[derive(Clone, Copy, Debug)]
pub enum SdResponse {
    /// Single-word response
    One(u32),
    /// Four-word response
    Four([u32; 4]),
}

/// List of SD card commands
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u32)]
pub enum SdCommandNumber {
    /// GO_IDLE_STATE
    ///
    /// Resets the SD card
    Cmd0 = 0,
    /// ALL_SEND_CID
    ///
    /// Requests card's unique card ID
    Cmd2 = 2,
    /// SEND_RELATIVE_ADDR
    ///
    /// Requests card's RCA
    Cmd3 = 3,
    /// SWITCH_FUNC
    ///
    /// Checks switchable function and/or switches card function
    Cmd6 = 6,
    /// SELECT/DESELECT_CARD
    ///
    /// Selects or deselects a card
    Cmd7 = 7,
    /// SEND_IF_COND
    ///
    /// Sends SD card interface conditions (voltage range)
    Cmd8 = 8,
    /// SEND_CSD
    ///
    /// Sends SD card-specific data
    Cmd9 = 9,
    /// SET_BLOCKLEN
    ///
    /// Sets SD card logical block length
    Cmd16 = 16,
    /// READ_SINGLE_BLOCK
    ///
    /// Reads a single block from the card
    Cmd17 = 17,
    /// SD_SEND_OP_COND
    ///
    /// Sends host capacity support info and requests card's operating
    /// conditions info
    Acmd41 = 41,
    /// SEND_SCR
    ///
    /// Requests SD configuration register
    Acmd51 = 51,
    /// APP_CMD
    ///
    /// Notifies the card that the following command is
    /// an "A-cmd" (application specific command)
    Cmd55 = 55,
}

/// Information structure for SD controller drivers
pub struct SdCommandInfo {
    /// Which response type to expect from this command
    pub response_type: SdResponseType,
}

/// Struct describing expected data transfer for a command
pub enum SdCommandTransfer<'a> {
    /// No transfer occurs
    None,
    /// Read from card (second param is block size)
    Read(&'a mut [u8], u16),
    /// Write to card (second param is block size)
    Write(&'a [u8], u16),
}

/// Generic SD card command structure
pub struct SdCommand<'a> {
    /// Command index
    pub number: SdCommandNumber,
    /// Argument value (0 if marked as 'stuff bits' in spec)
    pub argument: u32,
    /// Expected data transfer
    pub transfer: SdCommandTransfer<'a>,
}

impl SdCardStatus {
    /// Initial state for a SD card
    pub const fn invalid() -> Self {
        Self {
            phys_inserted: false,
            address: None,
            id: None,
        }
    }
}

impl SdResponseType {
    /// Returns `true` if response has 'busy' status
    pub const fn is_busy(self) -> bool {
        matches!(self, Self::R1b | Self::R5b)
    }
}

impl SdResponse {
    /// Returns single-word response or panics
    pub fn unwrap_one(&self) -> u32 {
        match *self {
            SdResponse::One(v) => v,
            _ => panic!("Unexpected response type"),
        }
    }

    /// Returns four-word response or panics
    pub fn unwrap_four(&self) -> [u32; 4] {
        match *self {
            SdResponse::Four(v) => v,
            _ => panic!("Unexpected response type"),
        }
    }
}

impl SdCommandInfo {
    /// Constructs a new command info struct
    pub const fn new(response_type: SdResponseType) -> Self {
        Self { response_type }
    }
}

impl SdCommand<'_> {
    /// Returns command information
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

    /// Returns `true` if cmd is application-specific
    pub const fn is_acmd(&self) -> bool {
        matches!(self.number, SdCommandNumber::Acmd41 | SdCommandNumber::Acmd51)
    }

    /// Returns the command index
    pub const fn number(&self) -> u32 {
        self.number as u32
    }
}
