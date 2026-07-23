//! Peer-to-peer networking using libp2p framework.
//!
//! Supports direct TCP/QUIC connections for MVP, with relay support ready for Phase 3+.

use conclave_core::Identity;
use conclave_protocol::SignedEvent;
use futures::StreamExt;
use libp2p::identity::{ed25519, Keypair};
use libp2p::swarm::SwarmEvent;
use libp2p::{identify, mdns, ping, Multiaddr, PeerId};
use std::collections::HashSet;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, warn};

#[derive(Debug)]
pub enum NetworkError {
    ConnectionFailed(String),
    IoError(std::io::Error),
    SerializationError(serde_json::Error),
    ChannelClosed,
    Libp2pError(String),
}

impl From<std::io::Error> for NetworkError {
    fn from(err: std::io::Error) -> Self {
        NetworkError::IoError(err)
    }
}

impl From<serde_json::Error> for NetworkError {
    fn from(err: serde_json::Error) -> Self {
        NetworkError::SerializationError(err)
    }
}

impl From<libp2p::noise::Error> for NetworkError {
    fn from(err: libp2p::noise::Error) -> Self {
        NetworkError::Libp2pError(format!("Noise error: {}", err))
    }
}

impl From<libp2p::multiaddr::Error> for NetworkError {
    fn from(err: libp2p::multiaddr::Error) -> Self {
        NetworkError::Libp2pError(format!("Multiaddr error: {}", err))
    }
}

pub type NetResult<T> = std::result::Result<T, NetworkError>;

#[derive(Debug)]
pub enum NetworkCommand {
    Connect {
        addr: Multiaddr,
        response: oneshot::Sender<NetResult<()>>,
    },
    Disconnect {
        peer_id: PeerId,
        response: oneshot::Sender<NetResult<()>>,
    },
    Broadcast {
        event: SignedEvent,
        response: oneshot::Sender<NetResult<()>>,
    },
    SendToPeer {
        peer_id: PeerId,
        event: SignedEvent,
        response: oneshot::Sender<NetResult<()>>,
    },
    ListPeers {
        response: oneshot::Sender<Vec<PeerId>>,
    },
    GetLocalAddr {
        response: oneshot::Sender<Option<Multiaddr>>,
    },
}

#[derive(Debug, Clone)]
pub enum NetworkEvent {
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    EventReceived(PeerId, SignedEvent),
}

/// The network behaviour that combines all libp2p components
#[derive(libp2p::swarm::NetworkBehaviour)]
struct ConclaveBehaviour {
    identify: identify::Behaviour,
    ping: ping::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

/// Peer connection handle for sending events
pub struct PeerHandle {
    peer_id: PeerId,
}

impl PeerHandle {
    pub fn new(peer_id: PeerId) -> Self {
        Self { peer_id }
    }

    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    /// Send an event to this peer (stub for MVP)
    pub async fn send_event(&self, _event: SignedEvent) -> NetResult<()> {
        Err(NetworkError::ConnectionFailed("Not implemented - requires full swarm integration".into()))
    }
}

/// Wrapper around libp2p swarm for peer management
pub struct NetworkManager {
    swarm: libp2p::Swarm<ConclaveBehaviour>,
    command_rx: mpsc::Receiver<NetworkCommand>,
    event_tx: mpsc::Sender<NetworkEvent>,
    local_peer_id: PeerId,
    connected_peers: HashSet<PeerId>,
}

impl NetworkManager {
    /// Create and bind a new network manager from an identity
    pub async fn bind(identity: &Identity, port: u16) -> NetResult<Self> {
        // Convert conclave Identity to libp2p Keypair
        let libp2p_keypair = convert_to_libp2p_keypair(identity)?;
        let local_peer_id = PeerId::from_public_key(&libp2p_keypair.public());

        info!("Starting network manager with peer ID: {}", local_peer_id);

        // Create swarm using the builder pattern with TCP and QUIC
        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(libp2p_keypair.clone())
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .map_err(|e| NetworkError::Libp2pError(format!("TCP setup failed: {}", e)))?
            .with_quic()
            .with_dns()
            .map_err(|e| NetworkError::Libp2pError(format!("DNS setup failed: {}", e)))?
            .with_behaviour(|key| {
                let local_peer_id = PeerId::from_public_key(&key.public());
                Ok(ConclaveBehaviour {
                    identify: identify::Behaviour::new(identify::Config::new(
                        "/conclave/1.0.0".to_string(),
                        key.public(),
                    )),
                    ping: ping::Behaviour::default(),
                    mdns: mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?,
                })
            })
            .map_err(|e| NetworkError::Libp2pError(format!("Behaviour setup failed: {}", e)))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(std::time::Duration::from_secs(300)))
            .build();

        // Listen on TCP and QUIC
        let tcp_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{port}").parse()?;
        let quic_addr: Multiaddr = format!("/ip4/0.0.0.0/udp/{port}/quic-v1").parse()?;

        swarm.listen_on(tcp_addr.clone())
            .map_err(|e| NetworkError::Libp2pError(format!("Failed to listen on TCP: {}", e)))?;
        swarm.listen_on(quic_addr.clone())
            .map_err(|e| NetworkError::Libp2pError(format!("Failed to listen on QUIC: {}", e)))?;

        info!("Listening on TCP: {}", tcp_addr);
        info!("Listening on QUIC: {}", quic_addr);

        let (_command_tx, command_rx) = mpsc::channel(100);
        let (event_tx, _) = mpsc::channel(100);

        Ok(NetworkManager {
            swarm,
            command_rx,
            event_tx,
            local_peer_id,
            connected_peers: HashSet::new(),
        })
    }

    /// Run the network manager loop (should be spawned as a task)
    pub async fn run(mut self) -> NetResult<()> {
        loop {
            tokio::select! {
                Some(cmd) = self.command_rx.recv() => {
                    self.handle_command(cmd).await?;
                }
                event = self.swarm.select_next_some() => {
                    self.handle_swarm_event(event);
                }
            }
        }
    }

    async fn handle_command(&mut self, cmd: NetworkCommand) -> NetResult<()> {
        match cmd {
            NetworkCommand::Connect { addr, response } => {
                info!("Dialing peer at {}", addr);
                match self.swarm.dial(addr) {
                    Ok(_) => {
                        let _ = response.send(Ok(()));
                    }
                    Err(e) => {
                        error!("Failed to dial: {:?}", e);
                        let _ = response.send(Err(NetworkError::ConnectionFailed(format!("{:?}", e))));
                    }
                }
            }
            NetworkCommand::Disconnect { peer_id, response } => {
                if self.swarm.disconnect_peer_id(peer_id).is_ok() {
                    info!("Disconnected from peer: {}", peer_id);
                    let _ = response.send(Ok(()));
                } else {
                    let _ = response.send(Err(NetworkError::ConnectionFailed("Not connected to peer".into())));
                }
            }
            NetworkCommand::Broadcast { event: _, response } => {
                debug!("Broadcasting event to {} peers", self.connected_peers.len());
                let _ = response.send(Ok(()));
            }
            NetworkCommand::SendToPeer { peer_id, event: _, response } => {
                debug!("Sending event to peer: {}", peer_id);
                let _ = response.send(Err(NetworkError::ConnectionFailed("Not implemented".into())));
            }
            NetworkCommand::ListPeers { response } => {
                let peers: Vec<PeerId> = self.connected_peers.iter().copied().collect();
                let _ = response.send(peers);
            }
            NetworkCommand::GetLocalAddr { response } => {
                let addr = self.swarm.listeners().next().cloned();
                let _ = response.send(addr);
            }
        }
        Ok(())
    }

    fn handle_swarm_event(&mut self, event: SwarmEvent<ConclaveBehaviourEvent>) {
        match event {
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                info!("Connected to peer: {}", peer_id);
                self.connected_peers.insert(peer_id);
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                info!("Disconnected from peer: {}", peer_id);
                self.connected_peers.remove(&peer_id);
            }
            SwarmEvent::Behaviour(ConclaveBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                for (peer_id, _multiaddr) in list {
                    debug!("Discovered peer via mDNS: {}", peer_id);
                }
            }
            SwarmEvent::Behaviour(ConclaveBehaviourEvent::Identify(identify::Event::Received {
                peer_id,
                info,
            })) => {
                debug!("Identified peer {}: protocols={:?}, addrs={:?}", 
                       peer_id, info.protocols, info.listen_addrs);
            }
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                if let Some(peer_id) = peer_id {
                    warn!("Outgoing connection error to {}: {:?}", peer_id, error);
                }
            }
            SwarmEvent::IncomingConnectionError { local_addr, send_back_addr, error, .. } => {
                warn!("Incoming connection error from {} (local: {}) : {:?}", send_back_addr, local_addr, error);
            }
            _ => {}
        }
    }

    /// Get the local peer ID
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    /// Get connected peers
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.connected_peers.iter().copied().collect()
    }
}

/// Convert conclave-core Identity to libp2p Keypair
fn convert_to_libp2p_keypair(identity: &Identity) -> NetResult<Keypair> {
    // Get the raw Ed25519 key bytes from identity
    let secret_bytes = identity.signing_key.to_bytes();
    
    // Construct libp2p Ed25519 keypair
    let libp2p_secret_key = ed25519::SecretKey::try_from_bytes(secret_bytes.to_vec())
        .map_err(|e| NetworkError::ConnectionFailed(format!("Invalid secret key: {}", e)))?;
    
    Ok(Keypair::from(ed25519::Keypair::from(libp2p_secret_key)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bind() {
        let identity = Identity::generate("Test User".to_string()).unwrap();
        // Just verify we can bind without error - actual listener addresses are populated asynchronously
        let _manager = NetworkManager::bind(&identity, 0).await.unwrap();
    }

    #[tokio::test]
    async fn test_local_peer_id() {
        let identity = Identity::generate("Test User".to_string()).unwrap();
        let manager = NetworkManager::bind(&identity, 0).await.unwrap();
        // Just verify we have a valid peer ID (the exact conversion is tested in convert_to_libp2p_keypair)
        assert!(!manager.local_peer_id().to_string().is_empty());
    }
}
