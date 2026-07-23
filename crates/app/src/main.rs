//! Conclave desktop application - MVP CLI

use clap::{Parser, Subcommand};
use conclave_core::Identity;
use conclave_network::{NetworkManager, CampaignDbHandle, NetworkCommand};
use conclave_plugin::PluginManager;
use conclave_protocol::{CampaignId, Event, MemberRole, SignedEvent};
use conclave_storage::{get_members, get_max_sequence, open_campaign_db, store_event};
use ed25519_dalek::SigningKey;

#[derive(Parser)]
#[command(name = "conclave")]
#[command(about = "Decentralized virtual tabletop for TTRPGs")]
struct Cli {
    /// Network port to listen on (default: 7777)
    #[arg(short, long, default_value = "7777")]
    port: u16,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize new identity (first run only)
    Init {
        /// Display name for this identity
        #[arg(short, long)]
        name: String,
    },

    /// Show current identity info
    Identity,

    /// Create a new campaign
    NewCampaign {
        /// Campaign name
        name: String,

        /// Optional rule set (e.g., "pathfinder-2e@1.0.0")
        #[arg(short, long)]
        rule_set: Option<String>,
    },

    /// Join an existing campaign
    JoinCampaign {
        /// Campaign ID (UUID)
        campaign_id: String,

        /// Peer address to connect to (host:port)
        peer_addr: String,
    },

    /// List available campaigns
    ListCampaigns,

    /// Send a chat message
    Chat {
        /// Campaign name
        campaign: String,

        /// Message content
        message: String,
    },

    /// Roll dice
    Roll {
        /// Dice expression (e.g., "2d20+5")
        expression: String,

        /// Campaign ID to record the roll in (optional)
        #[arg(short, long)]
        campaign: Option<String>,
    },

    /// List connected peers
    Peers,

    /// Connect to a peer manually
    Connect {
        /// Peer address (e.g., /ip4/192.168.1.100/tcp/7777)
        addr: String,
    },

    /// Start network listener (background mode)
    Listen,

    /// Load a plugin for a campaign
    LoadPlugin {
        /// Campaign ID
        campaign_id: String,

        /// Path to plugin library (.so, .dll, or .dylib)
        path: String,
    },

    /// Unload a plugin
    UnloadPlugin {
        /// Plugin name
        name: String,
    },

    /// List loaded plugins
    ListPlugins,

    /// Transfer DM authority to another player
    TransferDm {
        /// Campaign ID
        campaign_id: String,

        /// Target player ID (recipient's public key hex)
        target_player_id: String,
    },

    /// List members of a campaign (local database)
    Members {
        /// Campaign ID
        campaign_id: String,
    },

    /// Query remote peer for campaign members via RPC
    RpcMembers {
        /// Campaign ID
        campaign_id: String,

        /// Peer address to query (host:port)
        peer_addr: String,
    },

    /// Query remote peer for campaign info via RPC
    RpcInfo {
        /// Campaign ID
        campaign_id: String,

        /// Peer address to query (host:port)
        peer_addr: String,
    },

    /// Show current status and connected peers
    Status,

    /// Leave a campaign (broadcast MemberLeft event)
    LeaveCampaign {
        /// Campaign ID
        campaign_id: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let data_dir = dirs::data_dir().unwrap().join("conclave");
    std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");

    match cli.command {
        Commands::Init { name } => {
            println!("Generating new identity...");
            let identity = Identity::generate(name).expect("Failed to generate identity");
            
            // Save full identity with keys (in production, use encrypted storage)
            let id_path = data_dir.join("identity.json");
            serde_json::to_writer_pretty(
                std::fs::File::create(&id_path).unwrap(),
                &identity.to_json()
            ).unwrap();

            // Export mnemonic (in real app, show this ONCE and require confirmation)
            let mnemonic = identity.export_mnemonic();
            println!("\n⚠️  CRITICAL: Save this seed phrase securely!");
            println!("If you lose it, you cannot recover your identity.");
            println!("\nSeed phrase: {}", mnemonic);
            println!("\nPlayer ID: {}", identity.player_id());
        }

        Commands::Identity => {
            let id_path = data_dir.join("identity.json");
            if !id_path.exists() {
                println!("No identity found. Run 'conclave init --name YourName' first.");
                return;
            }

            let identity_json: serde_json::Value = serde_json::from_reader(
                std::fs::File::open(id_path).unwrap()
            ).unwrap();

            let identity = Identity::from_json(&identity_json).expect("Failed to load identity");
            println!("Player ID: {}", identity.player_id());
            println!("Display Name: {}", identity.display_name());
        }

        Commands::NewCampaign { name, rule_set } => {
            let id_path = data_dir.join("identity.json");
            if !id_path.exists() {
                println!("Error: No identity found. Run 'conclave init' first.");
                return;
            }

            let identity_json: serde_json::Value = serde_json::from_reader(
                std::fs::File::open(id_path).unwrap()
            ).unwrap();
            
            let identity = Identity::from_json(&identity_json).expect("Failed to load identity");
            let player_id = identity.player_id();

            // Create campaign database
            let campaign_uuid = uuid::Uuid::new_v4();
            let db_path = data_dir.join(format!("{}.db", campaign_uuid));
            
            let conn = open_campaign_db(&db_path).expect("Failed to create campaign DB");
            
            // Insert campaign record
            conn.execute(
                "INSERT INTO campaigns (id, name, dm_id, rule_set, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![campaign_uuid.to_string(), name, player_id, rule_set, chrono::Utc::now().timestamp()],
            ).expect("Failed to insert campaign");

            // Add DM as first member
            conclave_storage::add_member(
                &conn, 
                campaign_uuid, 
                player_id.clone(), 
                format!("{:?}", MemberRole::Dm)
            ).expect("Failed to add DM as member");

            // Create and store CampaignCreated event
            let mut key_bytes = [0u8; 32];
            key_bytes.copy_from_slice(&identity.signing_key.to_bytes());
            let signing_key = SigningKey::from_bytes(&key_bytes);

            let campaign_created_payload = serde_json::to_value(
                Event::CampaignCreated {
                    dm_id: player_id.clone(),
                    name: name.clone(),
                    rule_set,
                }
            ).unwrap();

            let mut campaign_event = SignedEvent::new(
                1,
                campaign_uuid,
                1,
                player_id.clone(),
                campaign_created_payload,
            );
            campaign_event.sign(&signing_key);

            store_event(&conn, &campaign_event).expect("Failed to store CampaignCreated event");

            println!("Campaign created: {}", campaign_uuid);
            println!("Share this ID with players so they can join.");
        }

        Commands::JoinCampaign { campaign_id, peer_addr } => {
            let id_path = data_dir.join("identity.json");
            if !id_path.exists() {
                println!("Error: No identity found. Run 'conclave init' first.");
                return;
            }

            let identity_json: serde_json::Value = serde_json::from_reader(
                std::fs::File::open(&id_path).unwrap()
            ).unwrap();
            
            let _identity = Identity::from_json(&identity_json).expect("Failed to load identity");

            // Parse campaign ID
            let campaign_uuid: CampaignId = campaign_id.parse().expect("Invalid campaign UUID");
            
            // Create campaign database if it doesn't exist
            let db_path = data_dir.join(format!("{}.db", campaign_id));
            let conn = open_campaign_db(&db_path).expect("Failed to create/open campaign DB");

            println!("Joining campaign {} via {}...", campaign_id, peer_addr);
            
            // Get local max sequence (0 if new campaign)
            let local_max_seq = get_max_sequence(&conn, campaign_uuid).unwrap_or(0);
            println!("Local has events up to sequence {}", local_max_seq);

            // Parse peer address
            let addr: libp2p::Multiaddr = format!("/ip4/{}/tcp/{}", 
                peer_addr.split(':').next().unwrap_or("127.0.0.1"),
                peer_addr.split(':').nth(1).unwrap_or("7777")
            ).parse().expect("Invalid peer address");

            println!("Connected as local peer");
            println!("Dialing peer at {}...", addr);

            let join_result = async {
                let (handle, mut manager) = match NetworkManager::bind(&_identity, 0).await {
                    Ok((h, m)) => (h, m),
                    Err(e) => {
                        eprintln!("Failed to start network: {}", e);
                        return Err(e);
                    }
                };

                manager.set_campaign_db(CampaignDbHandle::new(&db_path));

                tokio::spawn(async move {
                    if let Err(e) = manager.run().await {
                        eprintln!("Network error: {}", e);
                    }
                });

                let (tx, rx) = tokio::sync::oneshot::channel();
                handle.send_command(conclave_network::NetworkCommand::Connect { 
                    addr: addr.clone(), 
                    response: tx 
                }).await.unwrap();
                
                match rx.await {
                    Ok(Ok(())) => println!("Dial initiated"),
                    Ok(Err(e)) => return Err(e),
                    Err(_) => return Err(conclave_network::NetworkError::ConnectionFailed("Connection channel closed".into())),
                }

                let mut wait_count = 0;
                while wait_count < 20 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
                    let peers = handle.connected_peers().await;
                    if !peers.is_empty() {
                        println!("Connected to peer!");
                        break;
                    }
                    wait_count += 1;
                }

                let peers = handle.connected_peers().await;
                if peers.is_empty() {
                    return Err(conclave_network::NetworkError::ConnectionFailed("Failed to connect to peer".into()));
                }

                let identity_json: serde_json::Value = serde_json::from_reader(
                    std::fs::File::open(&id_path).unwrap()
                ).unwrap();
                let identity = Identity::from_json(&identity_json).expect("Failed to load identity");
                
                let mut key_bytes = [0u8; 32];
                key_bytes.copy_from_slice(&identity.signing_key.to_bytes());
                let signing_key = SigningKey::from_bytes(&key_bytes);

                let next_seq = local_max_seq + 1;
                
                let member_joined_payload = serde_json::to_value(
                    Event::MemberJoined {
                        player_id: identity.player_id(),
                        display_name: identity.display_name().to_string(),
                        role: MemberRole::Player,
                    }
                ).unwrap();

                let mut joined_event = SignedEvent::new(
                    next_seq,
                    campaign_uuid,
                    next_seq,
                    identity.player_id(),
                    member_joined_payload,
                );
                joined_event.sign(&signing_key);

                store_event(&conn, &joined_event).expect("Failed to store MemberJoined event");
                
                conclave_storage::add_member(
                    &conn, 
                    campaign_uuid, 
                    identity.player_id(), 
                    format!("{:?}", MemberRole::Player)
                ).expect("Failed to add member");
                
                println!("Broadcasting MemberJoined event (seq {})", next_seq);

                let _ = handle.send_command(conclave_network::NetworkCommand::Broadcast {
                    event: joined_event,
                    response: tokio::sync::oneshot::channel().0,
                }).await;

                let plugin_dir = data_dir.join("plugins").join(&campaign_id);
                if plugin_dir.exists() {
                    println!("Loading campaign plugins from {:?}", plugin_dir);
                    
                    let plugin_manager = PluginManager::new();
                    
                    for entry in std::fs::read_dir(&plugin_dir).unwrap() {
                        let entry = entry.unwrap();
                        let path = entry.path();
                        
                        if path.extension().and_then(|s| s.to_str()) == Some("so") ||
                           path.extension().and_then(|s| s.to_str()) == Some("dll") ||
                           path.extension().and_then(|s| s.to_str()) == Some("dylib") {
                            
                            let ctx = conclave_plugin::PluginContext::new(&campaign_id, &identity.player_id());
                            match plugin_manager.load_plugin_with_context(&path, ctx) {
                                Ok(name) => {
                                    println!("Loaded plugin: {} v{}", name, 
                                        plugin_manager.list_plugins().iter()
                                            .find(|(n, _)| n == &name)
                                            .map(|(_, v)| v.as_str())
                                            .unwrap_or("unknown")
                                    );

                                    let mut plugin_seq = next_seq;
                                    plugin_seq += 1;

                                    let plugin_loaded_payload = serde_json::to_value(
                                        Event::PluginLoaded {
                                            plugin_name: name.clone(),
                                            version: plugin_manager.list_plugins()
                                                .iter()
                                                .find(|(n, _)| n == &name)
                                                .map(|(_, v)| v.clone())
                                                .unwrap_or_default(),
                                        }
                                    ).unwrap();

                                    let mut plugin_event = SignedEvent::new(
                                        plugin_seq,
                                        campaign_uuid,
                                        plugin_seq,
                                        identity.player_id(),
                                        plugin_loaded_payload,
                                    );
                                    plugin_event.sign(&signing_key);

                                    store_event(&conn, &plugin_event).expect("Failed to store PluginLoaded event");
                                    
                                    let _ = handle.send_command(conclave_network::NetworkCommand::Broadcast {
                                        event: plugin_event,
                                        response: tokio::sync::oneshot::channel().0,
                                    }).await;
                                }
                                Err(e) => eprintln!("Failed to load plugin {:?}: {}", path, e),
                            }
                        }
                    }
                }

                println!("Requesting events from sequence {}...", local_max_seq + 1);
                
                let peers = handle.connected_peers().await;
                if peers.is_empty() {
                    return Err(conclave_network::NetworkError::ConnectionFailed("No connected peers to sync from".into()));
                }

                let target_peer = peers[0];
                match handle.sync_campaign_events(campaign_uuid, local_max_seq + 1, target_peer).await {
                    Ok(events) => {
                        println!("Received {} events from peer", events.len());
                        
                        for event in &events {
                            if !event.verify() {
                                eprintln!("Warning: Failed to verify event {}", event.id);
                                continue;
                            }
                            
                            store_event(&conn, event).expect("Failed to store event");
                            println!("Stored event {} (type: {}, seq: {})", 
                                event.id, event.event_type, event.sequence_number);
                        }
                        
                        println!("Sync complete! Campaign database updated.");
                    }
                    Err(e) => {
                        eprintln!("Failed to sync events: {}", e);
                        return Err(e);
                    }
                }

                let _ = handle.send_command(conclave_network::NetworkCommand::Disconnect { 
                    peer_id: target_peer, 
                    response: tokio::sync::oneshot::channel().0 
                }).await;
                
                Ok::<_, conclave_network::NetworkError>(())
            }.await;

            if let Err(e) = join_result {
                eprintln!("Join campaign failed: {}", e);
            }
        }

        Commands::ListCampaigns => {
            let campaigns_dir = data_dir;
            println!("Campaigns in {:?}:", campaigns_dir);
            
            for entry in std::fs::read_dir(&campaigns_dir).unwrap() {
                let entry = entry.unwrap();
                let name = entry.file_name().into_string().unwrap();
                if name.ends_with(".db") {
                    let campaign_id = &name[..name.len()-3];
                    
                    // Try to get campaign info from database
                    let db_path = campaigns_dir.join(&name);
                    let conn = match open_campaign_db(&db_path) {
                        Ok(c) => c,
                        Err(_) => {
                            println!("  - {} (corrupted)", campaign_id);
                            continue;
                        }
                    };
                    
                    let campaign_info: Option<(String, String)> = conn.query_row(
                        "SELECT name, dm_id FROM campaigns WHERE id = ?1",
                        rusqlite::params![campaign_id],
                        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    ).ok();
                    
                    if let Some((cam_name, dm_id)) = campaign_info {
                        println!("  - {} (name: {}, DM: {}...)", campaign_id, cam_name, &dm_id[..8]);
                    } else {
                        println!("  - {}", campaign_id);
                    }
                }
            }
        }

        Commands::Chat { campaign, message } => {
            let id_path = data_dir.join("identity.json");
            if !id_path.exists() {
                println!("Error: No identity found. Run 'conclave init' first.");
                return;
            }

            let identity_json: serde_json::Value = serde_json::from_reader(
                std::fs::File::open(&id_path).unwrap()
            ).unwrap();
            
            let identity = Identity::from_json(&identity_json).expect("Failed to load identity");

            // Parse campaign ID from name (for MVP, assume campaign filename is the UUID)
            let campaign_uuid: CampaignId = match campaign.parse() {
                Ok(id) => id,
                Err(_) => {
                    eprintln!("Invalid campaign ID format. Use the full UUID.");
                    return;
                }
            };

            let db_path = data_dir.join(format!("{}.db", campaign));
            if !db_path.exists() {
                eprintln!("Campaign database not found: {}", db_path.display());
                return;
            }

            let conn = open_campaign_db(&db_path).expect("Failed to open campaign DB");

            // Get next sequence number
            let next_seq = get_max_sequence(&conn, campaign_uuid).unwrap_or(0) + 1;

            let mut key_bytes = [0u8; 32];
            key_bytes.copy_from_slice(&identity.signing_key.to_bytes());
            let signing_key = SigningKey::from_bytes(&key_bytes);

            let chat_payload = serde_json::to_value(
                Event::ChatMessage {
                    author: identity.player_id(),
                    content: message.clone(),
                    character_name: None,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                }
            ).unwrap();

            let mut chat_event = SignedEvent::new(
                next_seq,
                campaign_uuid,
                next_seq,
                identity.player_id(),
                chat_payload,
            );
            chat_event.sign(&signing_key);

            store_event(&conn, &chat_event).expect("Failed to store chat event");
            println!("Chat message stored locally (seq {})", next_seq);
            println!("Note: Run 'conclave listen' in background to broadcast events to peers.");
        }

        Commands::Roll { expression, campaign } => {
            let id_path = data_dir.join("identity.json");
            if !id_path.exists() {
                println!("Error: No identity found. Run 'conclave init' first.");
                return;
            }

            let identity_json: serde_json::Value = serde_json::from_reader(
                std::fs::File::open(&id_path).unwrap()
            ).unwrap();
            
            let identity = Identity::from_json(&identity_json).expect("Failed to load identity");

            let (total, rolls) = roll_dice(&expression);
            println!("Rolled {}: {} (individual: {:?})", expression, total, rolls);

            if let Some(campaign_id) = campaign {
                let campaign_uuid: CampaignId = campaign_id.parse().expect("Invalid campaign UUID");
                let db_path = data_dir.join(format!("{}.db", campaign_id));
                
                if !db_path.exists() {
                    eprintln!("Campaign database not found: {}", db_path.display());
                    return;
                }

                let conn = open_campaign_db(&db_path).expect("Failed to open campaign DB");
                let next_seq = get_max_sequence(&conn, campaign_uuid).unwrap_or(0) + 1;

                let mut key_bytes = [0u8; 32];
                key_bytes.copy_from_slice(&identity.signing_key.to_bytes());
                let signing_key = SigningKey::from_bytes(&key_bytes);

                let dice_payload = serde_json::to_value(
                    Event::DiceRolled {
                        actor: identity.player_id(),
                        expression: expression.clone(),
                        result: total,
                        rolls: rolls.iter().map(|r| *r as i64).collect(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::SystemTime::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    }
                ).unwrap();

                let mut dice_event = SignedEvent::new(
                    next_seq,
                    campaign_uuid,
                    next_seq,
                    identity.player_id(),
                    dice_payload,
                );
                dice_event.sign(&signing_key);

                store_event(&conn, &dice_event).expect("Failed to store dice roll event");
                println!("Dice roll recorded in campaign {} (seq {})", campaign_id, next_seq);
            } else {
                println!("Note: Use --campaign <id> to record the roll in a campaign.");
            }
        }

        Commands::Peers => {
            println!("Network not active. Run 'conclave listen' first.");
        }

        Commands::Connect { addr } => {
            let id_path = data_dir.join("identity.json");
            if !id_path.exists() {
                println!("Error: No identity found. Run 'conclave init' first.");
                return;
            }

            let _identity_json: serde_json::Value = serde_json::from_reader(
                std::fs::File::open(&id_path).unwrap()
            ).unwrap();
            
            println!("Connecting to {}...", addr);
            // Note: Actual connection requires running NetworkManager loop
            // This is a stub - full implementation needs persistent manager
            println!("Note: Full peer connection requires 'conclave listen' to be running");
        }

        Commands::Listen => {
            let id_path = data_dir.join("identity.json");
            if !id_path.exists() {
                println!("Error: No identity found. Run 'conclave init' first.");
                return;
            }

            let identity_json: serde_json::Value = serde_json::from_reader(
                std::fs::File::open(&id_path).unwrap()
            ).unwrap();
            
            let identity = Identity::from_json(&identity_json).expect("Failed to load identity");
            
            println!("Starting network listener on port {}...", cli.port);
            println!("Player ID: {}", identity.player_id());
            println!("\nPress Ctrl+C to stop listening...\n");

            // Create and run NetworkManager
            let (handle, mut manager) = match NetworkManager::bind(&identity, cli.port).await {
                Ok((h, m)) => (h, m),
                Err(e) => {
                    eprintln!("Failed to start network: {}", e);
                    return;
                }
            };

            println!("Connected as peer {}", handle.local_peer_id());

            // Set up event callback to process incoming events
            let data_dir_clone = data_dir.clone();
            manager.set_event_callback(move |event| {
                println!("\n[EVENT RECEIVED] {} from campaign {}", event.event_type, event.campaign_id);
                
                let db_path = data_dir_clone.join(format!("{}.db", event.campaign_id));
                if !db_path.exists() {
                    eprintln!("Warning: Campaign DB not found for {}, ignoring event", event.campaign_id);
                    return;
                }

                let conn = match conclave_storage::open_campaign_db(&db_path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to open campaign DB: {}", e);
                        return;
                    }
                };

                // Verify signature before storing
                if !event.verify() {
                    eprintln!("Warning: Failed to verify event {}, rejecting", event.id);
                    return;
                }

                match conclave_storage::store_event(&conn, &event) {
                    Ok(()) => {
                        println!("Stored event {} (seq {})", event.id, event.sequence_number);
                        
                        // Handle MemberJoined events to update members table
                        if event.event_type == "MemberJoined" {
                            if let Some(player_id) = event.payload.get("player_id")
                                .and_then(|p| p.as_str()) 
                            {
                                if let Some(role_str) = event.payload.get("role")
                                    .and_then(|r| r.as_str())
                                {
                                    conn.execute(
                                        "INSERT OR REPLACE INTO members (campaign_id, player_id, role, joined_at) VALUES (?1, ?2, ?3, ?4)",
                                        rusqlite::params![event.campaign_id.to_string(), player_id, role_str, chrono::Utc::now().timestamp()],
                                    ).ok();
                                    println!("Added member: {} ({})", player_id, role_str);
                                }
                            }
                        }
                        
                        // Handle DmTransferred events to update roles
                        if event.event_type == "DmTransferred" {
                            if let (Some(from), Some(to)) = (
                                event.payload.get("from").and_then(|p| p.as_str()),
                                event.payload.get("to").and_then(|p| p.as_str())
                            ) {
                                conn.execute(
                                    "UPDATE members SET role = ?1 WHERE campaign_id = ?2 AND player_id = ?3",
                                    rusqlite::params!["Player", event.campaign_id.to_string(), from],
                                ).ok();
                                
                                conn.execute(
                                    "INSERT OR REPLACE INTO members (campaign_id, player_id, role, joined_at) VALUES (?1, ?2, ?3, ?4)",
                                    rusqlite::params![event.campaign_id.to_string(), to, "Dm", chrono::Utc::now().timestamp()],
                                ).ok();
                                
                                println!("DM transferred: {} -> {}", from, to);
                            }
                        }
                        
                        // Handle DiceRolled events for display
                        if event.event_type == "DiceRolled" {
                            if let (Some(actor), Some(expr), Some(result)) = (
                                event.payload.get("actor").and_then(|p| p.as_str()),
                                event.payload.get("expression").and_then(|p| p.as_str()),
                                event.payload.get("result").and_then(|p| p.as_i64())
                            ) {
                                println!("{} rolled {} = {}", &actor[..8], expr, result);
                            }
                        }
                        
                        // Handle ChatMessage events for display
                        if event.event_type == "ChatMessage" {
                            if let (Some(author), Some(content)) = (
                                event.payload.get("author").and_then(|p| p.as_str()),
                                event.payload.get("content").and_then(|p| p.as_str())
                            ) {
                                println!("{}: {}", &author[..8], content);
                            }
                        }
                    }
                    Err(e) => eprintln!("Failed to store event: {}", e),
                }
            });

            // Set up campaign DB for serving sync requests
            manager.set_campaign_db(CampaignDbHandle::new(data_dir.join("campaign.db")));

            println!("Listening on addresses:");
            for addr in manager.listening_addresses() {
                println!("  {}", addr);
            }
            println!();

            // Spawn the manager loop and wait for Ctrl+C
            tokio::spawn(async move {
                if let Err(e) = manager.run().await {
                    eprintln!("Network error: {}", e);
                }
            });

            println!("Use 'conclave status' to check network state");
            tokio::signal::ctrl_c().await.unwrap();
            println!("\nShutting down...");
        }

        Commands::LoadPlugin { campaign_id, path } => {
            let id_path = data_dir.join("identity.json");
            if !id_path.exists() {
                println!("Error: No identity found. Run 'conclave init' first.");
                return;
            }

            let identity_json: serde_json::Value = serde_json::from_reader(
                std::fs::File::open(&id_path).unwrap()
            ).unwrap();
            
            let identity = Identity::from_json(&identity_json).expect("Failed to load identity");

            let plugin_dir = data_dir.join("plugins").join(&campaign_id);
            std::fs::create_dir_all(&plugin_dir).expect("Failed to create plugin directory");

            let manager = PluginManager::new();
            
            let ctx = conclave_plugin::PluginContext::new(&campaign_id, &identity.player_id());
            
            match manager.load_plugin_with_context(&path, ctx) {
                Ok(name) => {
                    println!("Loaded plugin: {} v{}", name, 
                        manager.list_plugins().iter()
                            .find(|(n, _)| n == &name)
                            .map(|(_, v)| v.as_str())
                            .unwrap_or("unknown")
                    );

                    manager.associate_with_campaign(&campaign_id, &name)
                        .expect("Failed to associate plugin with campaign");

                    let plugins = manager.get_campaign_plugins(&campaign_id);
                    println!("Plugin associated with campaign {}. Active plugins: {:?}", 
                        campaign_id, plugins);
                }
                Err(e) => {
                    eprintln!("Failed to load plugin: {}", e);
                }
            }
        }

        Commands::UnloadPlugin { name } => {
            let manager = PluginManager::new();
            
            match manager.unload_plugin(&name) {
                Ok(()) => println!("Unloaded plugin: {}", name),
                Err(e) => eprintln!("Failed to unload plugin: {}", e),
            }
        }

        Commands::ListPlugins => {
            let manager = PluginManager::new();
            let plugins = manager.list_plugins();
            
            if plugins.is_empty() {
                println!("No plugins loaded.");
            } else {
                println!("Loaded plugins:");
                for (name, version) in plugins {
                    println!("  - {} v{}", name, version);
                }
            }
        }

        Commands::TransferDm { campaign_id, target_player_id } => {
            let id_path = data_dir.join("identity.json");
            if !id_path.exists() {
                println!("Error: No identity found. Run 'conclave init' first.");
                return;
            }

            let identity_json: serde_json::Value = serde_json::from_reader(
                std::fs::File::open(&id_path).unwrap()
            ).unwrap();
            
            let identity = Identity::from_json(&identity_json).expect("Failed to load identity");

            // Verify current user is DM by checking members table
            let campaign_uuid: CampaignId = campaign_id.parse().expect("Invalid campaign UUID");
            let db_path = data_dir.join(format!("{}.db", campaign_id));
            
            if !db_path.exists() {
                eprintln!("Campaign database not found: {}", db_path.display());
                return;
            }

            let conn = open_campaign_db(&db_path).expect("Failed to open campaign DB");
            let members = get_members(&conn, campaign_uuid).expect("Failed to get members");
            
            let current_is_dm = members.iter().any(|(pid, role)| 
                pid == &identity.player_id() && role == "Dm"
            );

            if !current_is_dm {
                eprintln!("Error: Only the current DM can transfer authority.");
                return;
            }

            let next_seq = get_max_sequence(&conn, campaign_uuid).unwrap_or(0) + 1;

            let mut key_bytes = [0u8; 32];
            key_bytes.copy_from_slice(&identity.signing_key.to_bytes());
            let signing_key = SigningKey::from_bytes(&key_bytes);

            let transfer_payload = serde_json::to_value(
                Event::DmTransferred {
                    from: identity.player_id(),
                    to: target_player_id.clone(),
                }
            ).unwrap();

            let mut transfer_event = SignedEvent::new(
                next_seq,
                campaign_uuid,
                next_seq,
                identity.player_id(),
                transfer_payload,
            );
            transfer_event.sign(&signing_key);

            store_event(&conn, &transfer_event).expect("Failed to store DM transfer event");
            
            // Update roles in members table
            conn.execute(
                "UPDATE members SET role = ?1 WHERE player_id = ?2",
                rusqlite::params!["Player", identity.player_id()],
            ).ok();
            
            conn.execute(
                "INSERT OR REPLACE INTO members (campaign_id, player_id, role, joined_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![campaign_id, target_player_id, "Dm", chrono::Utc::now().timestamp()],
            ).ok();

            println!("DM authority transferred to {}", target_player_id);
            println!("Note: Run 'conclave listen' in background to broadcast transfer to peers.");
        }

        Commands::Members { campaign_id } => {
            let db_path = data_dir.join(format!("{}.db", campaign_id));
            
            if !db_path.exists() {
                eprintln!("Campaign database not found: {}", db_path.display());
                return;
            }

            let campaign_uuid: CampaignId = campaign_id.parse().expect("Invalid campaign UUID");
            let conn = open_campaign_db(&db_path).expect("Failed to open campaign DB");
            
            let members = get_members(&conn, campaign_uuid).expect("Failed to get members");
            
            if members.is_empty() {
                println!("No members found in campaign {}", campaign_id);
            } else {
                println!("Members of campaign {}:", campaign_id);
                for (player_id, role) in members {
                    println!("  - {} ({})", &player_id[..8], role);
                }
            }
        }

        Commands::RpcMembers { campaign_id, peer_addr } => {
            let id_path = data_dir.join("identity.json");
            if !id_path.exists() {
                println!("Error: No identity found. Run 'conclave init' first.");
                return;
            }

            let identity_json: serde_json::Value = serde_json::from_reader(
                std::fs::File::open(&id_path).unwrap()
            ).unwrap();
            
            let identity = Identity::from_json(&identity_json).expect("Failed to load identity");

            println!("Querying peer {} for campaign {} members...", peer_addr, campaign_id);

            // Parse peer address
            let addr: libp2p::Multiaddr = format!("/ip4/{}/tcp/{}", 
                peer_addr.split(':').next().unwrap_or("127.0.0.1"),
                peer_addr.split(':').nth(1).unwrap_or("7777")
            ).parse().expect("Invalid peer address");

            println!("Querying peer {} for campaign {} members...", peer_addr, campaign_id);

            let rpc_result = async {
                let (handle, manager) = match NetworkManager::bind(&identity, 0).await {
                    Ok((h, m)) => (h, m),
                    Err(e) => {
                        eprintln!("Failed to start network: {}", e);
                        return Err(e);
                    }
                };

                println!("Connected as peer {}", handle.local_peer_id());
                println!("Dialing peer at {}...", addr);

                tokio::spawn(async move {
                    if let Err(e) = manager.run().await {
                        eprintln!("Network error: {}", e);
                    }
                });

                let (tx, rx) = tokio::sync::oneshot::channel();
                handle.send_command(NetworkCommand::Connect { 
                    addr: addr.clone(), 
                    response: tx 
                }).await.unwrap();
                
                match rx.await {
                    Ok(Ok(())) => println!("Dial initiated"),
                    Ok(Err(e)) => return Err(e),
                    Err(_) => return Err(conclave_network::NetworkError::ConnectionFailed("Connection channel closed".into())),
                }

                let mut wait_count = 0;
                while wait_count < 20 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
                    let peers = handle.connected_peers().await;
                    if !peers.is_empty() {
                        println!("Connected to peer!");
                        break;
                    }
                    wait_count += 1;
                }

                let peers = handle.connected_peers().await;
                if peers.is_empty() {
                    return Err(conclave_network::NetworkError::ConnectionFailed("No connected peers".into()));
                }

                let target_peer = peers[0];
                
                match handle.rpc_call(
                    target_peer, 
                    "get_members".to_string(), 
                    serde_json::json!({"campaign_id": campaign_id})
                ).await {
                    Ok(result) => {
                        if let Some(members) = result.as_array() {
                            println!("Members of campaign {}:", campaign_id);
                            for member in members {
                                if let (Some(pid), Some(role)) = (
                                    member.get(0).and_then(|v| v.as_str()),
                                    member.get(1).and_then(|v| v.as_str())
                                ) {
                                    println!("  - {} ({})", &pid[..8], role);
                                }
                            }
                        } else {
                            eprintln!("Unexpected response format");
                        }
                    }
                    Err(e) => {
                        eprintln!("RPC call failed: {}", e);
                        return Err(e);
                    }
                }

                let _ = handle.send_command(NetworkCommand::Disconnect { 
                    peer_id: target_peer, 
                    response: tokio::sync::oneshot::channel().0 
                }).await;
                
                Ok::<_, conclave_network::NetworkError>(())
            }.await;

            if let Err(e) = rpc_result {
                eprintln!("RPC failed: {}", e);
            }
        }

        Commands::RpcInfo { campaign_id, peer_addr } => {
            let id_path = data_dir.join("identity.json");
            if !id_path.exists() {
                println!("Error: No identity found. Run 'conclave init' first.");
                return;
            }

            let identity_json: serde_json::Value = serde_json::from_reader(
                std::fs::File::open(&id_path).unwrap()
            ).unwrap();
            
            let identity = Identity::from_json(&identity_json).expect("Failed to load identity");

            println!("Querying peer {} for campaign {} info...", peer_addr, campaign_id);

            // Parse peer address
            let addr: libp2p::Multiaddr = format!("/ip4/{}/tcp/{}", 
                peer_addr.split(':').next().unwrap_or("127.0.0.1"),
                peer_addr.split(':').nth(1).unwrap_or("7777")
            ).parse().expect("Invalid peer address");

            println!("Querying peer {} for campaign {} info...", peer_addr, campaign_id);

            let rpc_result = async {
                let (handle, manager) = match NetworkManager::bind(&identity, 0).await {
                    Ok((h, m)) => (h, m),
                    Err(e) => {
                        eprintln!("Failed to start network: {}", e);
                        return Err(e);
                    }
                };

                println!("Connected as peer {}", handle.local_peer_id());
                println!("Dialing peer at {}...", addr);

                tokio::spawn(async move {
                    if let Err(e) = manager.run().await {
                        eprintln!("Network error: {}", e);
                    }
                });

                let (tx, rx) = tokio::sync::oneshot::channel();
                handle.send_command(NetworkCommand::Connect { 
                    addr: addr.clone(), 
                    response: tx 
                }).await.unwrap();
                
                match rx.await {
                    Ok(Ok(())) => println!("Dial initiated"),
                    Ok(Err(e)) => return Err(e),
                    Err(_) => return Err(conclave_network::NetworkError::ConnectionFailed("Connection channel closed".into())),
                }

                let mut wait_count = 0;
                while wait_count < 20 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
                    let peers = handle.connected_peers().await;
                    if !peers.is_empty() {
                        println!("Connected to peer!");
                        break;
                    }
                    wait_count += 1;
                }

                let peers = handle.connected_peers().await;
                if peers.is_empty() {
                    return Err(conclave_network::NetworkError::ConnectionFailed("No connected peers".into()));
                }

                let target_peer = peers[0];
                
                match handle.rpc_call(
                    target_peer, 
                    "get_campaign_info".to_string(), 
                    serde_json::json!({"campaign_id": campaign_id})
                ).await {
                    Ok(result) => {
                        if let Some(name) = result.get("name").and_then(|v| v.as_str()) {
                            println!("Campaign: {}", name);
                        }
                        if let Some(dm_id) = result.get("dm_id").and_then(|v| v.as_str()) {
                            println!("DM: {}...", &dm_id[..8]);
                        }
                        if let Some(rule_set) = result.get("rule_set").and_then(|v| v.as_str()) {
                            println!("Rule set: {}", rule_set);
                        }
                    }
                    Err(e) => {
                        eprintln!("RPC call failed: {}", e);
                        return Err(e);
                    }
                }

                let _ = handle.send_command(NetworkCommand::Disconnect { 
                    peer_id: target_peer, 
                    response: tokio::sync::oneshot::channel().0 
                }).await;
                
                Ok::<_, conclave_network::NetworkError>(())
            }.await;

            if let Err(e) = rpc_result {
                eprintln!("RPC failed: {}", e);
            }
        }

        Commands::Status => {
            let id_path = data_dir.join("identity.json");
            
            println!("=== Conclave Status ===\n");
            
            if !id_path.exists() {
                println!("Identity: Not configured (run 'conclave init')");
            } else {
                let identity_json: serde_json::Value = serde_json::from_reader(
                    std::fs::File::open(&id_path).unwrap()
                ).unwrap();
                
                let identity = Identity::from_json(&identity_json).expect("Failed to load identity");
                println!("Identity: {} ({})", identity.display_name(), &identity.player_id()[..8]);
            }

            // Count campaigns
            let mut campaign_count = 0;
            for entry in std::fs::read_dir(&data_dir).unwrap() {
                if let Ok(entry) = entry {
                    let name = entry.file_name().into_string().unwrap();
                    if name.ends_with(".db") {
                        campaign_count += 1;
                    }
                }
            }
            println!("Campaigns: {}", campaign_count);

            // Count plugins
            let plugin_dir = data_dir.join("plugins");
            let mut plugin_count = 0;
            if plugin_dir.exists() {
                for entry in std::fs::read_dir(&plugin_dir).unwrap() {
                    if let Ok(entry) = entry {
                        if entry.path().is_dir() {
                            for plugin in std::fs::read_dir(entry.path()).unwrap() {
                                if let Ok(p) = plugin {
                                    if p.path().extension().and_then(|s| s.to_str()) == Some("so") ||
                                       p.path().extension().and_then(|s| s.to_str()) == Some("dll") ||
                                       p.path().extension().and_then(|s| s.to_str()) == Some("dylib") {
                                        plugin_count += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            println!("Plugins: {}", plugin_count);

            println!("\nNetwork: Not listening (run 'conclave listen' to start)");
        }

        Commands::LeaveCampaign { campaign_id } => {
            let id_path = data_dir.join("identity.json");
            if !id_path.exists() {
                println!("Error: No identity found. Run 'conclave init' first.");
                return;
            }

            let identity_json: serde_json::Value = serde_json::from_reader(
                std::fs::File::open(&id_path).unwrap()
            ).unwrap();
            
            let identity = Identity::from_json(&identity_json).expect("Failed to load identity");

            let campaign_uuid: CampaignId = campaign_id.parse().expect("Invalid campaign UUID");
            let db_path = data_dir.join(format!("{}.db", campaign_id));
            
            if !db_path.exists() {
                eprintln!("Campaign database not found: {}", db_path.display());
                return;
            }

            let conn = open_campaign_db(&db_path).expect("Failed to open campaign DB");
            let next_seq = get_max_sequence(&conn, campaign_uuid).unwrap_or(0) + 1;

            let mut key_bytes = [0u8; 32];
            key_bytes.copy_from_slice(&identity.signing_key.to_bytes());
            let signing_key = SigningKey::from_bytes(&key_bytes);

            // Create MemberLeft event
            let member_left_payload = serde_json::to_value(
                Event::MemberLeft {
                    player_id: identity.player_id(),
                }
            ).unwrap();

            let mut leave_event = SignedEvent::new(
                next_seq,
                campaign_uuid,
                next_seq,
                identity.player_id(),
                member_left_payload,
            );
            leave_event.sign(&signing_key);

            store_event(&conn, &leave_event).expect("Failed to store MemberLeft event");
            
            // Remove from members table
            conn.execute(
                "DELETE FROM members WHERE campaign_id = ?1 AND player_id = ?2",
                rusqlite::params![campaign_uuid.to_string(), identity.player_id()],
            ).ok();

            println!("Broadcasting leave for campaign {}", campaign_id);
            println!("Note: Run 'conclave listen' in background to broadcast to peers.");
        }
    }
}

fn roll_dice(expression: &str) -> (i64, Vec<i64>) {
    // Simple dice parser for MVP (e.g., "2d20+5")
    let parts: Vec<&str> = expression.split('+').collect();
    let mut total = 0;
    let mut all_rolls = Vec::new();

    for part in parts {
        if part.contains('d') {
            let dims: Vec<&str> = part.split('d').collect();
            if dims.len() == 2 {
                let count: i64 = dims[0].parse().unwrap_or(1);
                let sides: i64 = dims[1].parse().unwrap_or(20);
                for _ in 0..count {
                    let roll = (rand::random::<u32>() as i64 % sides) + 1;
                    total += roll;
                    all_rolls.push(roll);
                }
            }
        } else {
            let constant: i64 = part.parse().unwrap_or(0);
            total += constant;
        }
    }

    (total, all_rolls)
}
