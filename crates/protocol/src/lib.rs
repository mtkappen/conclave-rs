//! Conclave event protocol definitions.
//!
//! All game actions are represented as immutable, signed events.

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
            signature: String::new(), // Will be signed before storage
        }
    }

    /// Verify the event signature
    pub fn verify(&self) -> bool {
        // TODO: Implement Ed25519 verification
        true
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
}
