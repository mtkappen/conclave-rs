//! Conclave event protocol definitions.
//!
//! All game actions are represented as immutable, signed events.

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

/// Unique identifier for a campaign
pub type CampaignId = Uuid;

/// Unique identifier for a player (Ed25519 public key hex)
pub type PlayerId = String;

/// Unique identifier for an event within a campaign
pub type EventId = u64;

/// All possible event types in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Event {
    /// Campaign was created by DM
    CampaignCreated {
        dm_id: PlayerId,
        name: String,
        rule_set: Option<String>,
    },

    /// Player joined the campaign
    MemberJoined {
        player_id: PlayerId,
        display_name: String,
        role: MemberRole,
    },

    /// Player left the campaign
    MemberLeft {
        player_id: PlayerId,
    },

    /// Chat message sent by a player
    ChatMessage {
        author: PlayerId,
        content: String,
        character_name: Option<String>, // Optional in-character alias
        timestamp: u64,                  // Unix timestamp
    },

    /// Dice roll performed
    DiceRolled {
        actor: PlayerId,
        expression: String,  // e.g., "2d20+5"
        result: i64,
        rolls: Vec<i64>,     // Individual die results
        timestamp: u64,
    },

    /// DM transferred to another player
    DmTransferred {
        from: PlayerId,
        to: PlayerId,
    },

    /// Plugin loaded for campaign
    PluginLoaded {
        plugin_name: String,
        version: String,
    },

    /// Custom event from a plugin
    PluginEvent {
        plugin: String,
        event_type: String,
        data: serde_json::Value,
    },
}

/// Member role in a campaign
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemberRole {
    Dm,
    Player,
    Spectator,
}

/// An immutable event with metadata and signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedEvent {
    pub id: EventId,
    pub campaign_id: CampaignId,
    pub sequence_number: u64, // DM-assigned, monotonically increasing
    pub event_type: String,
    pub author_id: PlayerId,
    pub timestamp: SystemTime,
    pub payload: serde_json::Value,
    pub signature: String, // Ed25519 signature hex
}

impl SignedEvent {
    /// Create a new signed event (signature should be added by caller)
    pub fn new(
        id: EventId,
        campaign_id: CampaignId,
        sequence_number: u64,
        author_id: PlayerId,
        payload: serde_json::Value,
    ) -> Self {
        let event_type = match payload.get("type").and_then(|t| t.as_str()) {
            Some(t) => t.to_string(),
            None => "unknown".to_string(),
        };

        SignedEvent {
            id,
            campaign_id,
            sequence_number,
            event_type,
            author_id,
            timestamp: SystemTime::now(),
            payload,
            signature: String::new(),
        }
    }

    /// Sign the event with a private key
    pub fn sign(&mut self, signing_key: &SigningKey) {
        let data = self.signature_data();
        let signature = signing_key.sign(&data);
        self.signature = hex::encode(signature.to_bytes());
    }

    /// Verify the event signature using the author's public key
    pub fn verify(&self) -> bool {
        if self.signature.is_empty() {
            return false;
        }

        let sig_bytes = match hex::decode(&self.signature) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };

        let verifying_key_bytes = match hex::decode(&self.author_id) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };

        if verifying_key_bytes.len() != 32 {
            return false;
        }

        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(&verifying_key_bytes);
        
        let verifying_key = match VerifyingKey::from_bytes(&key_array) {
            Ok(k) => k,
            Err(_) => return false,
        };

        let sig = match ed25519_dalek::Signature::from_slice(&sig_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let data = self.signature_data();
        verifying_key.verify(&data, &sig).is_ok()
    }

    /// Get the data that should be signed (excludes signature itself)
    fn signature_data(&self) -> Vec<u8> {
        format!(
            "{}:{}:{}:{}:{}:{}:{}",
            self.id,
            self.campaign_id,
            self.sequence_number,
            self.event_type,
            self.author_id,
            self.timestamp.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
            serde_json::to_string(&self.payload).unwrap_or_default()
        ).into_bytes()
    }

    /// Get the author's public key as bytes
    pub fn author_public_key(&self) -> Option<[u8; 32]> {
        let bytes = hex::decode(&self.author_id).ok()?;
        if bytes.len() != 32 {
            return None;
        }
        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Some(array)
    }
}

/// Campaign metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campaign {
    pub id: CampaignId,
    pub name: String,
    pub dm_id: PlayerId,
    pub rule_set: Option<String>,
    pub created_at: SystemTime,
}

/// Plugin manifest structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub core_version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub verified: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = Event::ChatMessage {
            author: "abc123".to_string(),
            content: "Hello!".to_string(),
            character_name: Some("Gandalf".to_string()),
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: Event = serde_json::from_str(&json).unwrap();

        assert!(matches!(deserialized, Event::ChatMessage { .. }));
    }

    #[test]
    fn test_sign_and_verify_event() {
        use ed25519_dalek::SigningKey;
        use rand::RngCore;

        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key_bytes);
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key = signing_key.verifying_key();
        
        let player_id = hex::encode(verifying_key.as_bytes());
        
        let payload = serde_json::json!({
            "type": "ChatMessage",
            "author": player_id,
            "content": "Test message",
            "timestamp": 1234567890u64
        });

        let mut event = SignedEvent::new(
            1,
            Uuid::new_v4(),
            1,
            player_id.clone(),
            payload,
        );

        assert!(!event.verify()); // Not signed yet

        event.sign(&signing_key);

        assert!(event.verify()); // Should verify now
    }

    #[test]
    fn test_verify_fails_with_wrong_signature() {
        use ed25519_dalek::SigningKey;
        use rand::RngCore;

        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key_bytes);
        let signing_key = SigningKey::from_bytes(&key_bytes);
        
        let verifying_key = signing_key.verifying_key();
        let player_id = hex::encode(verifying_key.as_bytes());
        
        let payload = serde_json::json!({
            "type": "ChatMessage",
            "author": player_id,
            "content": "Test message",
            "timestamp": 1234567890u64
        });

        let mut event = SignedEvent::new(
            1,
            Uuid::new_v4(),
            1,
            player_id.clone(),
            payload,
        );

        event.sign(&signing_key);

        // Tamper with the signature
        event.signature.push('x');

        assert!(!event.verify()); // Should fail verification
    }

    #[test]
    fn test_verify_fails_with_tampered_data() {
        use ed25519_dalek::SigningKey;
        use rand::RngCore;

        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key_bytes);
        let signing_key = SigningKey::from_bytes(&key_bytes);
        
        let verifying_key = signing_key.verifying_key();
        let player_id = hex::encode(verifying_key.as_bytes());
        
        let payload = serde_json::json!({
            "type": "ChatMessage",
            "author": player_id,
            "content": "Test message",
            "timestamp": 1234567890u64
        });

        let mut event = SignedEvent::new(
            1,
            Uuid::new_v4(),
            1,
            player_id.clone(),
            payload,
        );

        event.sign(&signing_key);

        // Tamper with the content after signing
        if let Some(content) = event.payload.get_mut("content") {
            *content = serde_json::json!("Tampered message");
        }

        assert!(!event.verify()); // Should fail verification
    }
}
