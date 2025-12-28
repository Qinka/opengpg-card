//! OpenPGP card implementation
//!
//! Implements the OpenPGP card specification v3.4 command handlers
//! and state management.

use crate::apdu::{ApduCommand, ApduResponse, ApduInstruction, StatusWord};
use crate::crypto::{Algorithm, CryptoEngine, KeyPair, KeyType};
use crate::pin::{PinManager, PinType};
use crate::storage::{Storage, KeyId, DataId};
use heapless::Vec;

/// OpenPGP Application Identifier (AID)
const OPENPGP_AID: &[u8] = b"\xD2\x76\x00\x01\x24\x01";

/// Maximum response data size
const MAX_RESPONSE_SIZE: usize = 4096;

/// OpenPGP card state and command handler
pub struct OpenPgpCard {
    storage: Storage,
    pin_manager: PinManager,
    crypto_engine: CryptoEngine,
    selected: bool,
    // Key storage
    signature_key: Option<KeyPair>,
    decryption_key: Option<KeyPair>,
    authentication_key: Option<KeyPair>,
}

impl OpenPgpCard {
    /// Create a new OpenPGP card instance
    pub fn new(storage: Storage) -> Self {
        Self {
            storage,
            pin_manager: PinManager::new(),
            crypto_engine: CryptoEngine::new(),
            selected: false,
            signature_key: None,
            decryption_key: None,
            authentication_key: None,
        }
    }
    
    /// Process an APDU command
    pub fn process_apdu(&mut self, command: &ApduCommand) -> ApduResponse {
        // Check if card is selected for most operations
        if !self.selected && command.instruction() != Some(ApduInstruction::Select) {
            return ApduResponse::error(StatusWord::ConditionsNotSatisfied);
        }
        
        match command.instruction() {
            Some(ApduInstruction::Select) => self.handle_select(command),
            Some(ApduInstruction::GetData) => self.handle_get_data(command),
            Some(ApduInstruction::PutData) => self.handle_put_data(command),
            Some(ApduInstruction::Verify) => self.handle_verify(command),
            Some(ApduInstruction::ChangeReferenceData) => self.handle_change_reference_data(command),
            Some(ApduInstruction::ResetRetryCounter) => self.handle_reset_retry_counter(command),
            Some(ApduInstruction::GenerateAsymmetricKeyPair) => self.handle_generate_key(command),
            Some(ApduInstruction::InternalAuthenticate) => self.handle_internal_authenticate(command),
            Some(ApduInstruction::PsoComputeDigitalSignature) => self.handle_sign(command),
            Some(ApduInstruction::PsoDecipher) => self.handle_decipher(command),
            Some(ApduInstruction::GetChallenge) => self.handle_get_challenge(command),
            _ => ApduResponse::error(StatusWord::InsNotSupported),
        }
    }
    
    /// Handle SELECT command
    fn handle_select(&mut self, command: &ApduCommand) -> ApduResponse {
        // Check if selecting OpenPGP application
        if command.data.as_slice() == OPENPGP_AID {
            self.selected = true;
            log::info!("OpenPGP application selected");
            ApduResponse::new(&[])
        } else {
            ApduResponse::error(StatusWord::FileNotFound)
        }
    }
    
    /// Handle GET DATA command
    fn handle_get_data(&mut self, command: &ApduCommand) -> ApduResponse {
        let tag = ((command.p1 as u16) << 8) | (command.p2 as u16);
        
        log::info!("GET DATA: tag=0x{:04X}", tag);
        
        match tag {
            0x004F => {
                // Application Identifier
                ApduResponse::new(OPENPGP_AID)
            }
            0x5E => {
                // Login data
                match self.storage.load_data(DataId::LoginData) {
                    Ok(data) => ApduResponse::new(&data),
                    Err(_) => ApduResponse::new(&[]),
                }
            }
            0x5B => {
                // Cardholder name
                match self.storage.load_data(DataId::CardholderName) {
                    Ok(data) => ApduResponse::new(&data),
                    Err(_) => ApduResponse::new(&[]),
                }
            }
            0x5F50 => {
                // URL for public key
                match self.storage.load_data(DataId::PublicKeyUrl) {
                    Ok(data) => ApduResponse::new(&data),
                    Err(_) => ApduResponse::new(&[]),
                }
            }
            0x7F21 => {
                // Cardholder certificate
                ApduResponse::new(&[])
            }
            _ => ApduResponse::error(StatusWord::FileNotFound),
        }
    }
    
    /// Handle PUT DATA command
    fn handle_put_data(&mut self, command: &ApduCommand) -> ApduResponse {
        // Require admin PIN verification
        if !self.pin_manager.is_verified(PinType::Admin) {
            return ApduResponse::error(StatusWord::ConditionsNotSatisfied);
        }
        
        let tag = ((command.p1 as u16) << 8) | (command.p2 as u16);
        
        log::info!("PUT DATA: tag=0x{:04X}, len={}", tag, command.data.len());
        
        let data_id = match tag {
            0x5E => DataId::LoginData,
            0x5B => DataId::CardholderName,
            0x5F50 => DataId::PublicKeyUrl,
            _ => return ApduResponse::error(StatusWord::FileNotFound),
        };
        
        match self.storage.store_data(data_id, command.data.as_slice()) {
            Ok(_) => ApduResponse::new(&[]),
            Err(_) => ApduResponse::error(StatusWord::UnknownError),
        }
    }
    
    /// Handle VERIFY command
    fn handle_verify(&mut self, command: &ApduCommand) -> ApduResponse {
        let pin_type = match PinType::from_p2(command.p2) {
            Some(pt) => pt,
            None => return ApduResponse::error(StatusWord::IncorrectParameters),
        };
        
        if command.data.is_empty() {
            // Check PIN status
            if self.pin_manager.is_verified(pin_type) {
                return ApduResponse::new(&[]);
            } else {
                let retries = self.pin_manager.get_retries(pin_type);
                return ApduResponse::error(StatusWord::VerificationFailed);
            }
        }
        
        match self.pin_manager.verify(pin_type, command.data.as_slice()) {
            Ok(_) => ApduResponse::new(&[]),
            Err(crate::pin::PinError::Blocked) => {
                ApduResponse::error(StatusWord::AuthenticationBlocked)
            }
            Err(crate::pin::PinError::IncorrectPin(_)) => {
                ApduResponse::error(StatusWord::VerificationFailed)
            }
            Err(_) => ApduResponse::error(StatusWord::UnknownError),
        }
    }
    
    /// Handle CHANGE REFERENCE DATA command
    fn handle_change_reference_data(&mut self, command: &ApduCommand) -> ApduResponse {
        let pin_type = match PinType::from_p2(command.p2) {
            Some(pt) => pt,
            None => return ApduResponse::error(StatusWord::IncorrectParameters),
        };
        
        // Data should contain: old_pin || new_pin
        if command.data.len() < 12 {
            return ApduResponse::error(StatusWord::WrongLength);
        }
        
        let old_pin = &command.data[0..6];
        let new_pin = &command.data[6..];
        
        match self.pin_manager.change_pin(pin_type, old_pin, new_pin) {
            Ok(_) => ApduResponse::new(&[]),
            Err(_) => ApduResponse::error(StatusWord::VerificationFailed),
        }
    }
    
    /// Handle RESET RETRY COUNTER command
    fn handle_reset_retry_counter(&mut self, command: &ApduCommand) -> ApduResponse {
        if command.data.len() < 14 {
            return ApduResponse::error(StatusWord::WrongLength);
        }
        
        let admin_pin = &command.data[0..8];
        let new_user_pin = &command.data[8..];
        
        match self.pin_manager.reset_retry_counter(admin_pin, new_user_pin) {
            Ok(_) => ApduResponse::new(&[]),
            Err(_) => ApduResponse::error(StatusWord::VerificationFailed),
        }
    }
    
    /// Handle GENERATE ASYMMETRIC KEY PAIR command
    fn handle_generate_key(&mut self, command: &ApduCommand) -> ApduResponse {
        // Require admin PIN
        if !self.pin_manager.is_verified(PinType::Admin) {
            return ApduResponse::error(StatusWord::ConditionsNotSatisfied);
        }
        
        // P1 contains key reference (B6=Sign, B8=Decrypt, A4=Auth)
        let key_type = match command.p1 {
            0xB6 => KeyType::Signature,
            0xB8 => KeyType::Decryption,
            0xA4 => KeyType::Authentication,
            _ => return ApduResponse::error(StatusWord::IncorrectParameters),
        };
        
        // Default to Ed25519 for now
        let algorithm = Algorithm::Ed25519;
        
        log::info!("Generating {:?} key with {:?}", key_type, algorithm);
        
        match KeyPair::generate(algorithm, key_type) {
            Ok(keypair) => {
                // Store key
                let key_id = match key_type {
                    KeyType::Signature => KeyId::SignatureKey,
                    KeyType::Decryption => KeyId::DecryptionKey,
                    KeyType::Authentication => KeyId::AuthenticationKey,
                };
                
                // Export public key
                let public_key = match keypair.export_public_key() {
                    Ok(pk) => pk,
                    Err(_) => return ApduResponse::error(StatusWord::UnknownError),
                };
                
                // Store keypair in memory
                match key_type {
                    KeyType::Signature => self.signature_key = Some(keypair),
                    KeyType::Decryption => self.decryption_key = Some(keypair),
                    KeyType::Authentication => self.authentication_key = Some(keypair),
                }
                
                ApduResponse::new(&public_key)
            }
            Err(_) => ApduResponse::error(StatusWord::UnknownError),
        }
    }
    
    /// Handle INTERNAL AUTHENTICATE command
    fn handle_internal_authenticate(&mut self, command: &ApduCommand) -> ApduResponse {
        // Require user PIN
        if !self.pin_manager.is_verified(PinType::User) {
            return ApduResponse::error(StatusWord::ConditionsNotSatisfied);
        }
        
        let key = match &self.authentication_key {
            Some(k) => k,
            None => return ApduResponse::error(StatusWord::ConditionsNotSatisfied),
        };
        
        match self.crypto_engine.authenticate(key, command.data.as_slice()) {
            Ok(signature) => ApduResponse::new(&signature),
            Err(_) => ApduResponse::error(StatusWord::UnknownError),
        }
    }
    
    /// Handle PSO: COMPUTE DIGITAL SIGNATURE command
    fn handle_sign(&mut self, command: &ApduCommand) -> ApduResponse {
        // Check for signature operation
        if command.p1 != 0x9E || command.p2 != 0x9A {
            return ApduResponse::error(StatusWord::IncorrectParameters);
        }
        
        // Require user PIN
        if !self.pin_manager.is_verified(PinType::User) {
            return ApduResponse::error(StatusWord::ConditionsNotSatisfied);
        }
        
        let key = match &self.signature_key {
            Some(k) => k,
            None => return ApduResponse::error(StatusWord::ConditionsNotSatisfied),
        };
        
        match self.crypto_engine.sign(key, command.data.as_slice()) {
            Ok(signature) => {
                // Reset PIN verification after signing (OpenPGP card behavior)
                self.pin_manager.reset_verification(PinType::User);
                ApduResponse::new(&signature)
            }
            Err(_) => ApduResponse::error(StatusWord::UnknownError),
        }
    }
    
    /// Handle PSO: DECIPHER command
    fn handle_decipher(&mut self, command: &ApduCommand) -> ApduResponse {
        // Check for decipher operation
        if command.p1 != 0x80 || command.p2 != 0x86 {
            return ApduResponse::error(StatusWord::IncorrectParameters);
        }
        
        // Require user PIN
        if !self.pin_manager.is_verified(PinType::User) {
            return ApduResponse::error(StatusWord::ConditionsNotSatisfied);
        }
        
        let key = match &self.decryption_key {
            Some(k) => k,
            None => return ApduResponse::error(StatusWord::ConditionsNotSatisfied),
        };
        
        match self.crypto_engine.decrypt(key, command.data.as_slice()) {
            Ok(plaintext) => ApduResponse::new(&plaintext),
            Err(_) => ApduResponse::error(StatusWord::UnknownError),
        }
    }
    
    /// Handle GET CHALLENGE command
    fn handle_get_challenge(&mut self, command: &ApduCommand) -> ApduResponse {
        let size = command.le.unwrap_or(32).min(256);
        
        match self.crypto_engine.generate_challenge(size) {
            Ok(challenge) => ApduResponse::new(&challenge),
            Err(_) => ApduResponse::error(StatusWord::UnknownError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Tests would require mocking the Storage component
    // For embedded, testing is typically done via integration tests
}
