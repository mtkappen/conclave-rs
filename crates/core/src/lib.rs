//! Core game engine and identity management.

use bip39::{Language, Mnemonic};
use conclave_protocol::PlayerId;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::RngCore;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Failed to generate identity: {0}")]
    IdentityGeneration(String),

    #[error("Invalid mnemonic: {0}")]
    InvalidMnemonic(String),

    #[error("Signature verification failed")]
    VerificationFailed,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Hex decode error: {0}")]
    HexDecodeError(hex::FromHexError),
}

impl From<hex::FromHexError> for CoreError {
    fn from(err: hex::FromHexError) -> Self {
        CoreError::HexDecodeError(err)
    }
}

pub type Result<T> = std::result::Result<T, CoreError>;

/// User identity (Ed25519 keypair)
#[derive(Debug)]
pub struct Identity {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
    pub display_name: String,
}

impl Identity {
    /// Generate a new identity with random seed phrase
    pub fn generate(display_name: String) -> Result<Self> {
        let mut entropy = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut entropy);
        
        let mnemonic = Mnemonic::from_entropy(&entropy)
            .map_err(|e| CoreError::IdentityGeneration(e.to_string()))?;
        
        let seed: [u8; 64] = mnemonic.to_seed("");
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&seed[..32]);
        
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key = signing_key.verifying_key();

        Ok(Identity {
            signing_key,
            verifying_key,
            display_name,
        })
    }

    /// Restore identity from mnemonic seed phrase
    pub fn from_mnemonic(mnemonic: &str) -> Result<Self> {
        let parsed = Mnemonic::parse_in(Language::English, mnemonic)
            .map_err(|e| CoreError::InvalidMnemonic(e.to_string()))?;

        let seed: [u8; 64] = parsed.to_seed("");
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&seed[..32]);
        
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key = signing_key.verifying_key();

        Ok(Identity {
            signing_key,
            verifying_key,
            display_name: "Unknown".to_string(), // Name not recoverable from seed
        })
    }

    /// Get the player ID (public key hex)
    pub fn player_id(&self) -> PlayerId {
        hex::encode(self.verifying_key.as_bytes())
    }

    /// Sign data with private key
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        let signature = self.signing_key.sign(message);
        signature.to_bytes().to_vec()
    }

    /// Export mnemonic seed phrase for backup
    pub fn export_mnemonic(&self) -> String {
        // Derive entropy from signing key (simplified - in production use proper derivation)
        let bytes = self.signing_key.to_bytes();
        let mnemonic = Mnemonic::from_entropy(&bytes[..16]).unwrap(); // First 16 bytes for 12 words
        mnemonic.to_string()
    }

    /// Get display name
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Export identity to JSON (includes secret key - use secure storage!)
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "player_id": self.player_id(),
            "display_name": self.display_name,
            "signing_key_bytes": hex::encode(self.signing_key.to_bytes()),
        })
    }

    /// Load identity from JSON
    pub fn from_json(value: &serde_json::Value) -> Result<Self> {
        let signing_key_bytes = hex::decode(value["signing_key_bytes"].as_str()
            .ok_or(CoreError::IdentityGeneration("Missing signing_key_bytes".into()))?)?;
        
        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(&signing_key_bytes);
        
        let signing_key = SigningKey::from_bytes(&key_array);
        let verifying_key = signing_key.verifying_key();
        
        Ok(Identity {
            signing_key,
            verifying_key,
            display_name: value["display_name"].as_str().unwrap_or("Unknown").to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Verifier;

    #[test]
    fn test_generate_identity() {
        let identity = Identity::generate("Test User".to_string()).unwrap();
        assert!(!identity.player_id().is_empty());
        assert_eq!(identity.display_name(), "Test User");
    }

    #[test]
    fn test_sign_verify() {
        let identity = Identity::generate("Test User".to_string()).unwrap();
        let message = b"Hello, world!";
        let signature = identity.sign(message);

        // Verify using public key
        let sig = ed25519_dalek::Signature::from_slice(&signature).unwrap();
        let result = identity.verifying_key.verify(message, &sig);
        assert!(result.is_ok());
    }
}
