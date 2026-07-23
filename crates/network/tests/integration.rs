//! Integration tests for peer-to-peer networking

use conclave_core::Identity;
use conclave_network::{NetworkManager, NetworkCommand};
use std::time::Duration;

/// Test that two network managers can be created and bind to different ports
#[tokio::test]
async fn test_two_managers_bind() {
    let identity1 = Identity::generate("Peer 1".to_string()).unwrap();
    let identity2 = Identity::generate("Peer 2".to_string()).unwrap();

    let manager1 = NetworkManager::bind(&identity1, 0).await.unwrap();
    let manager2 = NetworkManager::bind(&identity2, 0).await.unwrap();

    // Both managers should have valid peer IDs
    assert!(!manager1.local_peer_id().to_string().is_empty());
    assert!(!manager2.local_peer_id().to_string().is_empty());

    // Peer IDs should be different
    assert_ne!(manager1.local_peer_id(), manager2.local_peer_id());
}

/// Test that we can get listening addresses from a manager
#[tokio::test]
async fn test_get_listening_addresses() {
    let identity = Identity::generate("Test".to_string()).unwrap();
    let manager = NetworkManager::bind(&identity, 0).await.unwrap();

    // Get listening addresses via public method
    let _addrs = manager.listening_addresses();
    
    // At minimum, we should have bound successfully without error
}

/// Test peer discovery via mDNS on localhost
#[tokio::test]
async fn test_mdns_discovery() {
    let identity1 = Identity::generate("Peer 1".to_string()).unwrap();
    let identity2 = Identity::generate("Peer 2".to_string()).unwrap();

    let _manager1 = NetworkManager::bind(&identity1, 0).await.unwrap();
    let _manager2 = NetworkManager::bind(&identity2, 0).await.unwrap();

    // Wait a bit for mDNS discovery
    tokio::time::sleep(Duration::from_secs(2)).await;

    // TODO: Verify peer1 discovered peer2 via mDNS
    // This requires implementing event handling in the network manager
}

/// Test manual connection between two peers
#[tokio::test]
async fn test_manual_connect() {
    let identity1 = Identity::generate("Peer 1".to_string()).unwrap();
    let identity2 = Identity::generate("Peer 2".to_string()).unwrap();

    // Bind both managers
    let _manager1 = NetworkManager::bind(&identity1, 0).await.unwrap();
    let manager2 = NetworkManager::bind(&identity2, 0).await.unwrap();

    // Get manager2's listening address
    let addrs = manager2.listening_addresses();
    
    if let Some(addr) = addrs.first() {
        // Try to construct a connect command
        let (tx, _rx) = tokio::sync::oneshot::channel();
        
        // Note: This test verifies we can construct the command
        // Actual connection requires running the network manager loop
        let _cmd = NetworkCommand::Connect {
            addr: addr.clone(),
            response: tx,
        };

        // TODO: Actually run the managers and verify connection
    }
}
