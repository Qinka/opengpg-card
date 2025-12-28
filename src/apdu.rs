//! APDU (Application Protocol Data Unit) command and response handling
//! 
//! Implements ISO/IEC 7816-4 APDU protocol for OpenPGP card communication.

use heapless::Vec;

/// Maximum APDU command length
pub const MAX_APDU_LEN: usize = 4096;

/// APDU Class byte values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApduClass {
    /// Standard ISO 7816-4 class
    Standard = 0x00,
    /// OpenPGP card proprietary class
    Proprietary = 0x80,
}

/// APDU Instruction codes for OpenPGP card
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ApduInstruction {
    Select = 0xA4,
    GetData = 0xCA,
    PutData = 0xDA,
    Verify = 0x20,
    ChangeReferenceData = 0x24,
    ResetRetryCounter = 0x2C,
    GenerateAsymmetricKeyPair = 0x47,
    InternalAuthenticate = 0x88,
    PsoComputeDigitalSignature = 0x2A,
    PsoDecipher = 0x2A,
    GetChallenge = 0x84,
    Terminate = 0xE6,
    Activate = 0x44,
}

impl ApduInstruction {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0xA4 => Some(Self::Select),
            0xCA => Some(Self::GetData),
            0xDA => Some(Self::PutData),
            0x20 => Some(Self::Verify),
            0x24 => Some(Self::ChangeReferenceData),
            0x2C => Some(Self::ResetRetryCounter),
            0x47 => Some(Self::GenerateAsymmetricKeyPair),
            0x88 => Some(Self::InternalAuthenticate),
            0x2A => Some(Self::PsoComputeDigitalSignature),
            0x84 => Some(Self::GetChallenge),
            0xE6 => Some(Self::Terminate),
            0x44 => Some(Self::Activate),
            _ => None,
        }
    }
}

/// APDU Status Word values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum StatusWord {
    Success = 0x9000,
    MoreDataAvailable = 0x6100,
    VerificationFailed = 0x6300,
    AuthenticationBlocked = 0x6983,
    ConditionsNotSatisfied = 0x6985,
    CommandNotAllowed = 0x6986,
    IncorrectParameters = 0x6A80,
    FileNotFound = 0x6A82,
    WrongLength = 0x6700,
    InsNotSupported = 0x6D00,
    ClaNotSupported = 0x6E00,
    UnknownError = 0x6F00,
}

impl StatusWord {
    pub fn to_bytes(self) -> [u8; 2] {
        let val = self as u16;
        [(val >> 8) as u8, (val & 0xFF) as u8]
    }
}

/// APDU Command structure
#[derive(Debug)]
pub struct ApduCommand {
    pub cla: u8,
    pub ins: u8,
    pub p1: u8,
    pub p2: u8,
    pub data: Vec<u8, MAX_APDU_LEN>,
    pub le: Option<usize>,
}

impl ApduCommand {
    /// Parse an APDU command from raw bytes
    pub fn parse(bytes: &[u8]) -> Result<Self, StatusWord> {
        if bytes.len() < 4 {
            return Err(StatusWord::WrongLength);
        }

        let cla = bytes[0];
        let ins = bytes[1];
        let p1 = bytes[2];
        let p2 = bytes[3];

        let mut data = Vec::new();
        let mut le = None;

        match bytes.len() {
            4 => {
                // Case 1: No data, no response expected
            }
            5 => {
                // Case 2s: No data, response expected
                le = Some(bytes[4] as usize);
            }
            len if len > 5 => {
                let lc = bytes[4] as usize;
                if len < 5 + lc {
                    return Err(StatusWord::WrongLength);
                }

                // Extract command data
                for i in 0..lc {
                    data.push(bytes[5 + i]).map_err(|_| StatusWord::WrongLength)?;
                }

                // Check for Le
                if len == 5 + lc + 1 {
                    le = Some(bytes[5 + lc] as usize);
                }
            }
            _ => return Err(StatusWord::WrongLength),
        }

        Ok(Self {
            cla,
            ins,
            p1,
            p2,
            data,
            le,
        })
    }

    /// Get the instruction as an enum
    pub fn instruction(&self) -> Option<ApduInstruction> {
        ApduInstruction::from_u8(self.ins)
    }
}

/// APDU Response builder
#[derive(Debug)]
pub struct ApduResponse {
    data: Vec<u8, MAX_APDU_LEN>,
    sw: StatusWord,
}

impl ApduResponse {
    /// Create a new successful response with data
    pub fn new(data: &[u8]) -> Self {
        let mut vec = Vec::new();
        vec.extend_from_slice(data).ok();
        Self {
            data: vec,
            sw: StatusWord::Success,
        }
    }

    /// Create an error response
    pub fn error(sw: StatusWord) -> Self {
        Self {
            data: Vec::new(),
            sw,
        }
    }

    /// Build the response bytes
    pub fn build(&self) -> Vec<u8, MAX_APDU_LEN> {
        let mut response = Vec::new();
        response.extend_from_slice(&self.data).ok();
        let sw_bytes = self.sw.to_bytes();
        response.extend_from_slice(&sw_bytes).ok();
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_case1_apdu() {
        let bytes = [0x00, 0xA4, 0x04, 0x00];
        let cmd = ApduCommand::parse(&bytes).unwrap();
        assert_eq!(cmd.cla, 0x00);
        assert_eq!(cmd.ins, 0xA4);
        assert_eq!(cmd.p1, 0x04);
        assert_eq!(cmd.p2, 0x00);
        assert_eq!(cmd.data.len(), 0);
        assert_eq!(cmd.le, None);
    }

    #[test]
    fn test_status_word_bytes() {
        let sw = StatusWord::Success;
        assert_eq!(sw.to_bytes(), [0x90, 0x00]);
    }
}
