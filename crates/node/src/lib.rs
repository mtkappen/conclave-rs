//! Optional relay node for Conclave-rs.
//!
//! # Architecture
//!
//! The node is NOT a central server and does NOT own campaigns. It is simply
//! another peer that usually stays online and provides availability improvements:
//!
//! - **Relay traffic**: Forward messages between peers that cannot connect directly
//! - **Campaign cache**: Store a subset of event logs for faster sync (not authoritative)
//! - **Peer synchronization**: Help offline peers catch up when they reconnect
//! - **Asset caching**: Cache files for faster distribution to peers
//! - **Optional services**: Transcription, AI summaries, NPC extraction (future)
//!
//! # Critical Design Constraint
//!
//! The node does NOT have authority over campaign data. Campaign state is owned
//! entirely by the peers (players and DM). The node is a convenience layer that
//! improves availability but is not required for campaign integrity.
//!
//! # Implementation Phases
//!
//! Phase 3+ (Post-MVP):
//! - Basic relay functionality using libp2p identify/relay protocols
//! - Campaign event cache with configurable retention
//! - Peer sync on connect/disconnect events
//!
//! Future:
//! - Asset caching and distribution
//! - Transcription service (voice → text)
//! - AI features (summaries, quest tracking)

use conclave_protocol::SignedEvent;
use std::collections::HashMap;
use thiserror::Error;
use tracing::{debug, info, warn};

#[derive(Error, Debug)]
pub enum NodeError {
    #[error("Node not initialized")]
    NotInitialized,

    #[error("Campaign not found: {0}")]
    CampaignNotFound(String),

    #[error("Relay error: {0}")]
    RelayError(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, NodeError>;

/// A relay node that forwards traffic between peers
pub struct RelayNode {
    /// Node's own peer ID
    peer_id: String,
    
    /// Connected peers and their last seen timestamps
    connected_peers: HashMap<String, u64>,
    
    /// Campaign event caches (campaign_id -> events)
    campaign_cache: HashMap<String, Vec<SignedEvent>>,
    
    /// Whether this node is currently active
    active: bool,
}

impl RelayNode {
    /// Create a new relay node with the given peer ID
    pub fn new(peer_id: String) -> Self {
        info!("Creating relay node with peer ID: {}", peer_id);
        
        Self {
            peer_id,
            connected_peers: HashMap::new(),
            campaign_cache: HashMap::new(),
            active: false,
        }
    }

    /// Start the relay node
    pub fn start(&mut self) -> Result<()> {
        if self.active {
            return Ok(());
        }
        
        info!("Starting relay node: {}", self.peer_id);
        self.active = true;
        Ok(())
    }

    /// Stop the relay node
    pub fn stop(&mut self) -> Result<()> {
        if !self.active {
            return Ok(());
        }
        
        info!("Stopping relay node: {}", self.peer_id);
        self.active = false;
        Ok(())
    }

    /// Check if the node is currently active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Register a peer connection
    pub fn peer_connected(&mut self, peer_id: String) -> Result<()> {
        debug!("Peer connected: {}", peer_id);
        self.connected_peers.insert(peer_id, std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs());
        Ok(())
    }

    /// Register a peer disconnection
    pub fn peer_disconnected(&mut self, peer_id: &str) -> Result<()> {
        debug!("Peer disconnected: {}", peer_id);
        self.connected_peers.remove(peer_id);
        Ok(())
    }

    /// Get list of currently connected peers
    pub fn connected_peers(&self) -> Vec<String> {
        self.connected_peers.keys().cloned().collect()
    }

    /// Cache events for a campaign (optional, for faster sync)
    pub fn cache_events(&mut self, campaign_id: String, events: Vec<SignedEvent>) -> Result<()> {
        if !self.active {
            return Err(NodeError::NotInitialized);
        }
        
        debug!("Caching {} events for campaign {}", events.len(), campaign_id);
        self.campaign_cache.insert(campaign_id, events);
        Ok(())
    }

    /// Get cached events for a campaign
    pub fn get_cached_events(&self, campaign_id: &str) -> Option<&Vec<SignedEvent>> {
        self.campaign_cache.get(campaign_id)
    }

    /// Relay an event from one peer to others
    pub async fn relay_event(&self, _event: SignedEvent, _target_peers: &[String]) -> Result<()> {
        if !self.active {
            return Err(NodeError::NotInitialized);
        }
        
        // TODO: Implement actual relay logic using libp2p
        warn!("Relay event not yet implemented - node is placeholder only");
        Ok(())
    }

    /// Sync events to a peer that just connected
    pub async fn sync_to_peer(&self, _peer_id: &str, _campaign_id: &str) -> Result<Vec<SignedEvent>> {
        if !self.active {
            return Err(NodeError::NotInitialized);
        }
        
        // TODO: Implement actual sync logic
        warn!("Sync to peer not yet implemented - node is placeholder only");
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_lifecycle() {
        let mut node = RelayNode::new("test-peer-id".to_string());
        
        assert!(!node.is_active());
        
        node.start().unwrap();
        assert!(node.is_active());
        
        node.stop().unwrap();
        assert!(!node.is_active());
    }

    #[test]
    fn test_peer_tracking() {
        let mut node = RelayNode::new("test-peer-id".to_string());
        node.start().unwrap();
        
        node.peer_connected("peer-1".to_string()).unwrap();
        node.peer_connected("peer-2".to_string()).unwrap();
        
        let peers = node.connected_peers();
        assert_eq!(peers.len(), 2);
        
        node.peer_disconnected("peer-1").unwrap();
        
        let peers = node.connected_peers();
        assert_eq!(peers.len(), 1);
    }

    #[test]
    fn test_event_caching() {
        let mut node = RelayNode::new("test-peer-id".to_string());
        node.start().unwrap();
        
        let events = vec![]; // Empty for now, would be SignedEvent in real usage
        node.cache_events("campaign-1".to_string(), events).unwrap();
        
        assert!(node.get_cached_events("campaign-1").is_some());
    }
}
