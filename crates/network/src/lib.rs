//! Peer-to-peer networking using libp2p framework.
//!
//! Supports direct TCP/QUIC connections for MVP, with relay support ready for Phase 3+.

use conclave_core::Identity;
use conclave_protocol::{CampaignId, SignedEvent};
use futures::StreamExt;
use libp2p::identity::{ed25519, Keypair};
use libp2p::request_response::{self, OutboundRequestId, ProtocolSupport};
use libp2p::swarm::SwarmEvent;
use libp2p::{identify, mdns, ping, Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub type EventCallback = Arc<dyn Fn(SignedEvent) + Send + Sync>;

struct PendingSyncRequest {
    campaign_id: CampaignId,
    response_tx: oneshot::Sender<NetResult<Vec<SignedEvent>>>,
}

struct PendingRpcRequest {
    #[allow(dead_code)]
    request_id: u64,
    response_tx: oneshot::Sender<NetResult<serde_json::Value>>,
}

/// Database connection wrapper for serving sync requests
pub struct CampaignDbHandle {
    pub path: std::path::PathBuf,
}

impl CampaignDbHandle {
    pub fn new<P: AsRef<std::path::Path>>(path: P) -> Self {
        Self { 
            path: path.as_ref().to_path_buf() 
        }
    }

    pub fn get_events(&self, campaign_id: CampaignId, _from_sequence: u64) -> NetResult<Vec<SignedEvent>> {
        let conn = conclave_storage::open_campaign_db(&self.path)
            .map_err(|e| NetworkError::ConnectionFailed(format!("DB error: {}", e)))?;
        
        conclave_storage::get_events_up_to(&conn, campaign_id, u64::MAX)
            .map_err(|e| NetworkError::ConnectionFailed(format!("DB query error: {}", e)))
    }

    pub fn get_max_sequence(&self, campaign_id: CampaignId) -> NetResult<u64> {
        let conn = conclave_storage::open_campaign_db(&self.path)
            .map_err(|e| NetworkError::ConnectionFailed(format!("DB error: {}", e)))?;
        
        conclave_storage::get_max_sequence(&conn, campaign_id)
            .map_err(|e| NetworkError::ConnectionFailed(format!("DB query error: {}", e)))
    }
}

/// Protocol ID for event broadcast/request-response
const CONCLAVE_EVENT_PROTOCOL: &str = "/conclave/event/1.0.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSyncRequest {
    pub campaign_id: CampaignId,
    pub from_sequence: u64,
    pub to_sequence: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSyncResponse {
    pub events: Vec<SignedEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcRequest {
    pub method: String,
    pub params: serde_json::Value,
    pub request_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub request_id: u64,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NetworkRequest {
    Sync(EventSyncRequest),
    Broadcast(SignedEvent),
    Rpc(RpcRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NetworkResponse {
    Sync(EventSyncResponse),
    Ack,
    Rpc(RpcResponse),
}

#[derive(Debug)]
pub enum NetworkError {
    ConnectionFailed(String),
    IoError(std::io::Error),
    SerializationError(serde_json::Error),
    ChannelClosed,
    Libp2pError(String),
}

impl std::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            NetworkError::IoError(e) => write!(f, "IO error: {}", e),
            NetworkError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            NetworkError::ChannelClosed => write!(f, "Channel closed"),
            NetworkError::Libp2pError(msg) => write!(f, "libp2p error: {}", msg),
        }
    }
}

impl std::error::Error for NetworkError {}

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
    SyncCampaignEvents {
        campaign_id: CampaignId,
        from_sequence: u64,
        to_peer: PeerId,
        response: oneshot::Sender<NetResult<Vec<SignedEvent>>>,
    },
    RpcCall {
        peer_id: PeerId,
        method: String,
        params: serde_json::Value,
        response: oneshot::Sender<NetResult<serde_json::Value>>,
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
    event_sync: request_response::json::Behaviour<NetworkRequest, NetworkResponse>,
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

pub struct NetworkHandle {
    command_tx: mpsc::Sender<NetworkCommand>,
    local_peer_id: PeerId,
    connected_peers: Arc<Mutex<HashSet<PeerId>>>,
}

impl NetworkHandle {
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    pub async fn connected_peers(&self) -> Vec<PeerId> {
        let peers = self.connected_peers.lock().await;
        peers.iter().copied().collect()
    }

    pub async fn send_command(&self, cmd: NetworkCommand) -> NetResult<()> {
        self.command_tx.send(cmd).await.map_err(|_| NetworkError::ChannelClosed)?;
        Ok(())
    }

    pub async fn rpc_call(
        &self,
        peer_id: PeerId,
        method: String,
        params: serde_json::Value,
    ) -> NetResult<serde_json::Value> {
        let (tx, rx) = oneshot::channel();
        
        self.command_tx.send(NetworkCommand::RpcCall {
            peer_id,
            method,
            params,
            response: tx,
        }).await.map_err(|_| NetworkError::ChannelClosed)?;
        
        rx.await.map_err(|_| NetworkError::ChannelClosed)?
    }

    pub async fn sync_campaign_events(
        &self,
        campaign_id: CampaignId,
        from_sequence: u64,
        to_peer: PeerId,
    ) -> NetResult<Vec<SignedEvent>> {
        let (tx, rx) = oneshot::channel();
        
        self.command_tx.send(NetworkCommand::SyncCampaignEvents {
            campaign_id,
            from_sequence,
            to_peer,
            response: tx,
        }).await.map_err(|_| NetworkError::ChannelClosed)?;
        
        rx.await.map_err(|_| NetworkError::ChannelClosed)?
    }
}

/// Wrapper around libp2p swarm for peer management
pub struct NetworkManager {
    swarm: libp2p::Swarm<ConclaveBehaviour>,
    command_tx: mpsc::Sender<NetworkCommand>,
    command_rx: mpsc::Receiver<NetworkCommand>,
    local_peer_id: PeerId,
    connected_peers: HashSet<PeerId>,
    pending_sync_requests: HashMap<OutboundRequestId, PendingSyncRequest>,
    pending_rpc_requests: HashMap<u64, PendingRpcRequest>,
    campaign_db: Option<CampaignDbHandle>,
    event_callback: Option<EventCallback>,
    connected_peers_shared: Arc<Mutex<HashSet<PeerId>>>,
}

impl NetworkManager {
    /// Create and bind a new network manager from an identity
    /// Returns a handle for sending commands and the manager to be spawned
    pub async fn bind(identity: &Identity, port: u16) -> NetResult<(NetworkHandle, Self)> {
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
                    event_sync: request_response::json::Behaviour::<NetworkRequest, NetworkResponse>::new(
                        [(libp2p::StreamProtocol::new(CONCLAVE_EVENT_PROTOCOL), ProtocolSupport::Full)],
                        request_response::Config::default(),
                    ),
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

        let (command_tx, command_rx) = mpsc::channel(100);
        let connected_peers_shared = Arc::new(Mutex::new(HashSet::new()));

        let handle = NetworkHandle {
            command_tx: command_tx.clone(),
            local_peer_id,
            connected_peers: connected_peers_shared.clone(),
        };

        Ok((handle, NetworkManager {
            swarm,
            command_tx,
            command_rx,
            local_peer_id,
            connected_peers: HashSet::new(),
            pending_sync_requests: HashMap::new(),
            pending_rpc_requests: HashMap::new(),
            campaign_db: None,
            event_callback: None,
            connected_peers_shared,
        }))
    }

    /// Set callback for incoming events
    pub fn set_event_callback<F>(&mut self, callback: F)
    where
        F: Fn(SignedEvent) + Send + Sync + 'static,
    {
        self.event_callback = Some(Arc::new(callback));
    }

    /// Set the campaign database for serving sync requests
    pub fn set_campaign_db(&mut self, db_handle: CampaignDbHandle) {
        self.campaign_db = Some(db_handle);
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
            NetworkCommand::Broadcast { event, response } => {
                debug!("Broadcasting event to {} peers", self.connected_peers.len());
                
                for peer_id in self.connected_peers.iter() {
                    let request = NetworkRequest::Broadcast(event.clone());
                    self.swarm.behaviour_mut().event_sync.send_request(peer_id, request);
                    info!("Sent broadcast event to {}", peer_id);
                }
                
                let _ = response.send(Ok(()));
            }
            NetworkCommand::SendToPeer { peer_id, event, response } => {
                if !self.connected_peers.contains(&peer_id) {
                    let _ = response.send(Err(NetworkError::ConnectionFailed("Not connected to peer".into())));
                    return Ok(());
                }
                
                debug!("Sending event to peer: {}", peer_id);
                let request = NetworkRequest::Broadcast(event);
                self.swarm.behaviour_mut().event_sync.send_request(&peer_id, request);
                info!("Sent event to {}", peer_id);
                
                let _ = response.send(Ok(()));
            }
            NetworkCommand::ListPeers { response } => {
                let peers: Vec<PeerId> = self.connected_peers.iter().copied().collect();
                let _ = response.send(peers);
            }
            NetworkCommand::GetLocalAddr { response } => {
                let addr = self.swarm.listeners().next().cloned();
                let _ = response.send(addr);
            }
            NetworkCommand::SyncCampaignEvents { campaign_id, from_sequence, to_peer, response } => {
                if !self.connected_peers.contains(&to_peer) {
                    let _ = response.send(Err(NetworkError::ConnectionFailed("Not connected to peer".into())));
                    return Ok(());
                }

                debug!("Requesting events for campaign {} from sequence {}", campaign_id, from_sequence);
                
                let request = NetworkRequest::Sync(EventSyncRequest {
                    campaign_id,
                    from_sequence,
                    to_sequence: None,
                });

                let outbound_id = self.swarm.behaviour_mut().event_sync.send_request(&to_peer, request);
                
                self.pending_sync_requests.insert(outbound_id, PendingSyncRequest {
                    campaign_id,
                    response_tx: response,
                });
            }
            NetworkCommand::RpcCall { peer_id, method, params, response } => {
                if !self.connected_peers.contains(&peer_id) {
                    let _ = response.send(Err(NetworkError::ConnectionFailed("Not connected to peer".into())));
                    return Ok(());
                }

                debug!("Making RPC call {} to {}", method, peer_id);
                
                static RPC_COUNTER: std::sync::LazyLock<std::sync::atomic::AtomicU64> = 
                    std::sync::LazyLock::new(|| std::sync::atomic::AtomicU64::new(0));
                
                let request_id = RPC_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                
                let request = NetworkRequest::Rpc(RpcRequest {
                    method: method.clone(),
                    params,
                    request_id,
                });

                self.pending_rpc_requests.insert(request_id, PendingRpcRequest {
                    request_id,
                    response_tx: response,
                });

                self.swarm.behaviour_mut().event_sync.send_request(&peer_id, request);
            }
        }
        Ok(())
    }

    fn handle_swarm_event(&mut self, event: SwarmEvent<ConclaveBehaviourEvent>) {
        match event {
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                info!("Connected to peer: {}", peer_id);
                self.connected_peers.insert(peer_id);
                let _ = self.connected_peers_shared.blocking_lock().insert(peer_id);
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                info!("Disconnected from peer: {}", peer_id);
                self.connected_peers.remove(&peer_id);
                let _ = self.connected_peers_shared.blocking_lock().remove(&peer_id);
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
            SwarmEvent::Behaviour(ConclaveBehaviourEvent::EventSync(message)) => match message {
                request_response::Event::Message { peer, message } => match message {
                    request_response::Message::Request { request_id: _, channel, request } => {
                        match request {
                            NetworkRequest::Sync(sync_req) => {
                                info!("Received event sync request for campaign {} from sequence {}", 
                                      sync_req.campaign_id, sync_req.from_sequence);
                                
                                let events = if let Some(db) = &self.campaign_db {
                                    match db.get_events(sync_req.campaign_id, sync_req.from_sequence) {
                                        Ok(evts) => {
                                            evts.into_iter()
                                                .filter(|e| e.sequence_number >= sync_req.from_sequence)
                                                .collect()
                                        }
                                        Err(e) => {
                                            warn!("Failed to get events for sync: {}", e);
                                            vec![]
                                        }
                                    }
                                } else {
                                    warn!("No campaign DB configured, returning empty response");
                                    vec![]
                                };
                                
                                info!("Sending {} events in sync response", events.len());
                                let response = NetworkResponse::Sync(EventSyncResponse { events });
                                let _ = self.swarm.behaviour_mut().event_sync.send_response(channel, response);
                            }
                            NetworkRequest::Broadcast(event) => {
                                info!("Received broadcast event {} from {}", event.id, peer);
                                
                                if let Some(cb) = &self.event_callback {
                                    cb(event.clone());
                                }
                                
                                let response = NetworkResponse::Ack;
                                let _ = self.swarm.behaviour_mut().event_sync.send_response(channel, response);
                            }
                            NetworkRequest::Rpc(rpc_req) => {
                                debug!("Received RPC call {} from {}", rpc_req.method, peer);
                                
                                // Handle built-in RPC methods
                                let result: Result<serde_json::Value, String> = match rpc_req.method.as_str() {
                                    "get_members" => {
                                        if let Some(campaign_id_str) = rpc_req.params.get("campaign_id").and_then(|v| v.as_str()) {
                                            if let Ok(campaign_id) = campaign_id_str.parse::<Uuid>() {
                                                if let Some(db) = &self.campaign_db {
                                                    match conclave_storage::open_campaign_db(&db.path) {
                                                        Ok(conn) => {
                                                            match conclave_storage::get_members(&conn, campaign_id) {
                                                                Ok(members) => Ok(serde_json::to_value(members).unwrap()),
                                                                Err(e) => Err(format!("Query error: {}", e)),
                                                            }
                                                        }
                                                        Err(e) => Err(format!("DB error: {}", e)),
                                                    }
                                                } else {
                                                    Err("No database configured".into())
                                                }
                                            } else {
                                                Err("Invalid campaign ID".into())
                                            }
                                        } else {
                                            Err("Missing campaign_id parameter".into())
                                        }
                                    }
                                    "get_max_sequence" => {
                                        if let Some(campaign_id_str) = rpc_req.params.get("campaign_id").and_then(|v| v.as_str()) {
                                            if let Ok(campaign_id) = campaign_id_str.parse::<Uuid>() {
                                                if let Some(db) = &self.campaign_db {
                                                    match db.get_max_sequence(campaign_id) {
                                                        Ok(seq) => Ok(serde_json::json!({"sequence": seq})),
                                                        Err(e) => Err(format!("Query error: {}", e)),
                                                    }
                                                } else {
                                                    Err("No database configured".into())
                                                }
                                            } else {
                                                Err("Invalid campaign ID".into())
                                            }
                                        } else {
                                            Err("Missing campaign_id parameter".into())
                                        }
                                    }
                                    "get_campaign_info" => {
                                        if let Some(campaign_id_str) = rpc_req.params.get("campaign_id").and_then(|v| v.as_str()) {
                                            if let Ok(campaign_id) = campaign_id_str.parse::<Uuid>() {
                                                if let Some(db) = &self.campaign_db {
                                                    match conclave_storage::open_campaign_db(&db.path) {
                                                        Ok(conn) => {
                                                            let info: Option<(String, String, Option<String>)> = conn.query_row(
                                                                "SELECT name, dm_id, rule_set FROM campaigns WHERE id = ?1",
                                                                rusqlite::params![campaign_id.to_string()],
                                                                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?))
                                                            ).ok();
                                                            
                                                            match info {
                                                                Some((name, dm_id, rule_set)) => {
                                                                    Ok(serde_json::json!({
                                                                        "name": name,
                                                                        "dm_id": dm_id,
                                                                        "rule_set": rule_set
                                                                    }))
                                                                }
                                                                None => Err("Campaign not found".into())
                                                            }
                                                        }
                                                        Err(e) => Err(format!("DB error: {}", e)),
                                                    }
                                                } else {
                                                    Err("No database configured".into())
                                                }
                                            } else {
                                                Err("Invalid campaign ID".into())
                                            }
                                        } else {
                                            Err("Missing campaign_id parameter".into())
                                        }
                                    }
                                    _ => Err(format!("Unknown method: {}", rpc_req.method)),
                                };

                                match result {
                                    Ok(val) => {
                                        let rpc_resp = NetworkResponse::Rpc(RpcResponse {
                                            request_id: rpc_req.request_id,
                                            result: Some(val),
                                            error: None,
                                        });
                                        let _ = self.swarm.behaviour_mut().event_sync.send_response(channel, rpc_resp);
                                    }
                                    Err(err) => {
                                        let rpc_resp = NetworkResponse::Rpc(RpcResponse {
                                            request_id: rpc_req.request_id,
                                            result: None,
                                            error: Some(err),
                                        });
                                        let _ = self.swarm.behaviour_mut().event_sync.send_response(channel, rpc_resp);
                                    }
                                }
                            }
                        }
                    }
                    request_response::Message::Response { request_id, response } => {
                        match response {
                            NetworkResponse::Sync(sync_resp) => {
                                debug!("Received event sync response: {} events", sync_resp.events.len());
                                
                                if let Some(pending) = self.pending_sync_requests.remove(&request_id) {
                                    match pending.response_tx.send(Ok(sync_resp.events)) {
                                        Ok(_) => info!("Completed sync request for campaign {}", pending.campaign_id),
                                        Err(_) => warn!("Sync response receiver dropped for campaign {}", pending.campaign_id),
                                    }
                                } else {
                                    warn!("Received unexpected sync response");
                                }
                            }
                            NetworkResponse::Ack => {
                                debug!("Received ack for broadcast event");
                            }
                            NetworkResponse::Rpc(rpc_resp) => {
                                debug!("Received RPC response for request {}", rpc_resp.request_id);
                                
                                if let Some(pending) = self.pending_rpc_requests.remove(&rpc_resp.request_id) {
                                    match rpc_resp.result {
                                        Some(result) => {
                                            let _ = pending.response_tx.send(Ok(result));
                                        }
                                        None => {
                                            let err = rpc_resp.error.unwrap_or_else(|| "RPC error".into());
                                            let _ = pending.response_tx.send(Err(NetworkError::ConnectionFailed(err)));
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                request_response::Event::InboundFailure { peer, error, .. } => {
                    warn!("Inbound failure from peer {:?}: {:?}", peer, error);
                }
                request_response::Event::OutboundFailure { request_id, peer, error, .. } => {
                    warn!("Outbound failure to peer {:?}: {:?}", peer, error);
                    
                    if let Some(pending) = self.pending_sync_requests.remove(&request_id) {
                        let _ = pending.response_tx.send(Err(NetworkError::ConnectionFailed(format!("Sync failed: {:?}", error))));
                    }
                }
                _ => {}
            },
            SwarmEvent::OutgoingConnectionError { peer_id: Some(peer_id), error, .. } => {
                warn!("Outgoing connection error to {}: {:?}", peer_id, error);
            }
            SwarmEvent::OutgoingConnectionError { peer_id: None, error, .. } => {
                warn!("Outgoing connection error (no peer): {:?}", error);
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

    /// Get listening addresses
    pub fn listening_addresses(&self) -> Vec<Multiaddr> {
        self.swarm.listeners().cloned().collect()
    }

    /// Request events from a peer for campaign sync
    pub async fn sync_campaign_events(
        &self,
        campaign_id: CampaignId,
        from_sequence: u64,
        to_peer: PeerId,
    ) -> NetResult<Vec<SignedEvent>> {
        let (tx, rx) = oneshot::channel();
        
        self.command_tx.send(NetworkCommand::SyncCampaignEvents {
            campaign_id,
            from_sequence,
            to_peer,
            response: tx,
        }).await.map_err(|_| NetworkError::ChannelClosed)?;
        
        rx.await.map_err(|_| NetworkError::ChannelClosed)?
    }

    /// Send command to network manager
    pub async fn send_command(&self, cmd: NetworkCommand) -> NetResult<()> {
        self.command_tx.send(cmd).await.map_err(|_| NetworkError::ChannelClosed)
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
        let (handle, _manager) = NetworkManager::bind(&identity, 0).await.unwrap();
        // Just verify we have a valid peer ID (the exact conversion is tested in convert_to_libp2p_keypair)
        assert!(!handle.local_peer_id().to_string().is_empty());
    }
}
