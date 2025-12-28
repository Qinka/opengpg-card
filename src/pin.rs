//! PIN verification and management
//!
//! Handles User PIN (PW1) and Admin PIN (PW3) verification, 
//! retry counters, and PIN change operations.

use heapless::Vec;

/// Maximum PIN length
pub const MAX_PIN_LEN: usize = 32;

/// Default User PIN (PW1) - "123456"
const DEFAULT_USER_PIN: &[u8] = b"123456";

/// Default Admin PIN (PW3) - "12345678"
const DEFAULT_ADMIN_PIN: &[u8] = b"12345678";

/// Maximum retry attempts for User PIN
const USER_PIN_MAX_RETRIES: u8 = 3;

/// Maximum retry attempts for Admin PIN
const ADMIN_PIN_MAX_RETRIES: u8 = 3;

/// PIN types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinType {
    /// User PIN (PW1) - for signing and general operations
    User = 0x81,
    /// Admin PIN (PW3) - for administrative operations
    Admin = 0x83,
}

impl PinType {
    pub fn from_p2(p2: u8) -> Option<Self> {
        match p2 {
            0x81 | 0x82 => Some(Self::User),
            0x83 => Some(Self::Admin),
            _ => None,
        }
    }
}

/// PIN state and verification logic
#[derive(Debug)]
pub struct PinManager {
    user_pin: Vec<u8, MAX_PIN_LEN>,
    admin_pin: Vec<u8, MAX_PIN_LEN>,
    user_pin_retries: u8,
    admin_pin_retries: u8,
    user_pin_verified: bool,
    admin_pin_verified: bool,
}

impl PinManager {
    /// Create a new PIN manager with default PINs
    pub fn new() -> Self {
        let mut user_pin = Vec::new();
        user_pin.extend_from_slice(DEFAULT_USER_PIN).ok();
        
        let mut admin_pin = Vec::new();
        admin_pin.extend_from_slice(DEFAULT_ADMIN_PIN).ok();

        Self {
            user_pin,
            admin_pin,
            user_pin_retries: USER_PIN_MAX_RETRIES,
            admin_pin_retries: ADMIN_PIN_MAX_RETRIES,
            user_pin_verified: false,
            admin_pin_verified: false,
        }
    }

    /// Verify a PIN
    pub fn verify(&mut self, pin_type: PinType, pin: &[u8]) -> Result<(), PinError> {
        match pin_type {
            PinType::User => {
                if self.user_pin_retries == 0 {
                    return Err(PinError::Blocked);
                }

                if pin == self.user_pin.as_slice() {
                    self.user_pin_verified = true;
                    self.user_pin_retries = USER_PIN_MAX_RETRIES;
                    Ok(())
                } else {
                    self.user_pin_retries = self.user_pin_retries.saturating_sub(1);
                    self.user_pin_verified = false;
                    Err(PinError::IncorrectPin(self.user_pin_retries))
                }
            }
            PinType::Admin => {
                if self.admin_pin_retries == 0 {
                    return Err(PinError::Blocked);
                }

                if pin == self.admin_pin.as_slice() {
                    self.admin_pin_verified = true;
                    self.admin_pin_retries = ADMIN_PIN_MAX_RETRIES;
                    Ok(())
                } else {
                    self.admin_pin_retries = self.admin_pin_retries.saturating_sub(1);
                    self.admin_pin_verified = false;
                    Err(PinError::IncorrectPin(self.admin_pin_retries))
                }
            }
        }
    }

    /// Check if a PIN is currently verified
    pub fn is_verified(&self, pin_type: PinType) -> bool {
        match pin_type {
            PinType::User => self.user_pin_verified,
            PinType::Admin => self.admin_pin_verified,
        }
    }

    /// Reset PIN verification status (e.g., after operation)
    pub fn reset_verification(&mut self, pin_type: PinType) {
        match pin_type {
            PinType::User => self.user_pin_verified = false,
            PinType::Admin => self.admin_pin_verified = false,
        }
    }

    /// Change a PIN (requires current PIN verification)
    pub fn change_pin(
        &mut self,
        pin_type: PinType,
        old_pin: &[u8],
        new_pin: &[u8],
    ) -> Result<(), PinError> {
        if new_pin.len() < 6 || new_pin.len() > MAX_PIN_LEN {
            return Err(PinError::InvalidLength);
        }

        // Verify old PIN first
        self.verify(pin_type, old_pin)?;

        // Set new PIN
        match pin_type {
            PinType::User => {
                self.user_pin.clear();
                self.user_pin
                    .extend_from_slice(new_pin)
                    .map_err(|_| PinError::InvalidLength)?;
            }
            PinType::Admin => {
                self.admin_pin.clear();
                self.admin_pin
                    .extend_from_slice(new_pin)
                    .map_err(|_| PinError::InvalidLength)?;
            }
        }

        Ok(())
    }

    /// Reset retry counter using Admin PIN
    pub fn reset_retry_counter(&mut self, admin_pin: &[u8], new_user_pin: &[u8]) -> Result<(), PinError> {
        if new_user_pin.len() < 6 || new_user_pin.len() > MAX_PIN_LEN {
            return Err(PinError::InvalidLength);
        }

        // Verify admin PIN
        self.verify(PinType::Admin, admin_pin)?;

        // Reset user PIN and counter
        self.user_pin.clear();
        self.user_pin
            .extend_from_slice(new_user_pin)
            .map_err(|_| PinError::InvalidLength)?;
        self.user_pin_retries = USER_PIN_MAX_RETRIES;
        self.user_pin_verified = false;

        Ok(())
    }

    /// Get remaining retry attempts
    pub fn get_retries(&self, pin_type: PinType) -> u8 {
        match pin_type {
            PinType::User => self.user_pin_retries,
            PinType::Admin => self.admin_pin_retries,
        }
    }
}

impl Default for PinManager {
    fn default() -> Self {
        Self::new()
    }
}

/// PIN-related errors
#[derive(Debug, Clone, Copy)]
pub enum PinError {
    /// PIN is incorrect, returns remaining attempts
    IncorrectPin(u8),
    /// PIN is blocked (no retries remaining)
    Blocked,
    /// Invalid PIN length
    InvalidLength,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_pin_verification() {
        let mut pm = PinManager::new();
        assert!(pm.verify(PinType::User, b"123456").is_ok());
        assert!(pm.is_verified(PinType::User));
    }

    #[test]
    fn test_incorrect_pin_reduces_retries() {
        let mut pm = PinManager::new();
        let result = pm.verify(PinType::User, b"wrong");
        match result {
            Err(PinError::IncorrectPin(retries)) => assert_eq!(retries, 2),
            _ => panic!("Expected IncorrectPin error"),
        }
    }

    #[test]
    fn test_pin_blocking() {
        let mut pm = PinManager::new();
        pm.user_pin_retries = 0;
        let result = pm.verify(PinType::User, b"123456");
        assert!(matches!(result, Err(PinError::Blocked)));
    }
}
