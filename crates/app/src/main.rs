//! Conclave desktop application - MVP CLI

use clap::{Parser, Subcommand};
use conclave_core::Identity;
use conclave_network::{NetworkManager, CampaignDbHandle};
use conclave_plugin::PluginManager;
use conclave_protocol::{CampaignId, Event, MemberRole, SignedEvent};
use conclave_storage::{get_max_sequence, open_campaign_db, store_event};
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
            let campaign_id = uuid::Uuid::new_v4();
            let db_path = data_dir.join(format!("{}.db", campaign_id));
            
            let conn = open_campaign_db(&db_path).expect("Failed to create campaign DB");
            
            // Insert campaign record
            conn.execute(
                "INSERT INTO campaigns (id, name, dm_id, rule_set, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![campaign_id.to_string(), name, player_id, rule_set, chrono::Utc::now().timestamp()],
            ).expect("Failed to insert campaign");

            println!("Campaign created: {}", campaign_id);
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

            // Create network manager and run sync in background
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let mut manager = match NetworkManager::bind(&_identity, 0).await {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("Failed to start network: {}", e);
                        return;
                    }
                };

                // Set up campaign DB for serving sync requests
                manager.set_campaign_db(CampaignDbHandle::new(&db_path));

                println!("Connected as peer {}", manager.local_peer_id());
                println!("Dialing peer at {}...", addr);

                // Connect to peer
                let (tx, rx) = tokio::sync::oneshot::channel();
                manager.send_command(conclave_network::NetworkCommand::Connect { 
                    addr: addr.clone(), 
                    response: tx 
                }).await.unwrap();
                
                match rx.await {
                    Ok(Ok(())) => println!("Connected to peer!"),
                    Ok(Err(e)) => {
                        eprintln!("Failed to connect: {}", e);
                        return;
                    }
                    Err(_) => {
                        eprintln!("Connection channel closed");
                        return;
                    }
                }

                // Wait a bit for connection to establish
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                // Create and broadcast MemberJoined event
                let identity_json: serde_json::Value = serde_json::from_reader(
                    std::fs::File::open(&id_path).unwrap()
                ).unwrap();
                let identity = Identity::from_json(&identity_json).expect("Failed to load identity");
                
                let mut key_bytes = [0u8; 32];
                key_bytes.copy_from_slice(&identity.signing_key.to_bytes());
                let signing_key = SigningKey::from_bytes(&key_bytes);

                // Get next sequence number
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

                // Store locally
                store_event(&conn, &joined_event).expect("Failed to store MemberJoined event");
                println!("Broadcasting MemberJoined event (seq {})", next_seq);

                // Broadcast the event
                let _ = manager.send_command(conclave_network::NetworkCommand::Broadcast {
                    event: joined_event,
                    response: tokio::sync::oneshot::channel().0,
                }).await;

                // Load campaign-specific plugins
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

                                    // Broadcast PluginLoaded event
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
                                    
                                    let _ = manager.send_command(conclave_network::NetworkCommand::Broadcast {
                                        event: plugin_event,
                                        response: tokio::sync::oneshot::channel().0,
                                    }).await;
                                }
                                Err(e) => eprintln!("Failed to load plugin {:?}: {}", path, e),
                            }
                        }
                    }
                }

                // Request events from peer
                println!("Requesting events from sequence {}...", local_max_seq + 1);
                
                let peers = manager.connected_peers();
                if peers.is_empty() {
                    eprintln!("No connected peers to sync from");
                    return;
                }

                let target_peer = peers[0];
                match manager.sync_campaign_events(campaign_uuid, local_max_seq + 1, target_peer).await {
                    Ok(events) => {
                        println!("Received {} events from peer", events.len());
                        
                        for event in &events {
                            // Verify signature before storing
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
                    }
                }

                // Disconnect and shutdown
                let _ = manager.send_command(conclave_network::NetworkCommand::Disconnect { 
                    peer_id: target_peer, 
                    response: tokio::sync::oneshot::channel().0 
                }).await;
            });
        }

        Commands::ListCampaigns => {
            let campaigns_dir = data_dir;
            println!("Campaigns in {:?}:", campaigns_dir);
            
            for entry in std::fs::read_dir(&campaigns_dir).unwrap() {
                let entry = entry.unwrap();
                let name = entry.file_name().into_string().unwrap();
                if name.ends_with(".db") {
                    println!("  - {}", &name[..name.len()-3]); // Remove .db extension
                }
            }
        }

        Commands::Chat { campaign, message } => {
            println!("Sending chat to '{}': {}", campaign, message);
            // TODO: Load campaign DB and insert event
        }

        Commands::Roll { expression } => {
            println!("Rolling {}: {}", expression, roll_dice(&expression));
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
            let manager = match NetworkManager::bind(&identity, cli.port).await {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Failed to start network: {}", e);
                    return;
                }
            };

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
    }
}

fn roll_dice(expression: &str) -> i64 {
    // Simple dice parser for MVP (e.g., "2d20+5")
    let parts: Vec<&str> = expression.split('+').collect();
    let mut total = 0;

    for part in parts {
        if part.contains('d') {
            let dims: Vec<&str> = part.split('d').collect();
            if dims.len() == 2 {
                let count: i64 = dims[0].parse().unwrap_or(1);
                let sides: i64 = dims[1].parse().unwrap_or(20);
                for _ in 0..count {
                    total += (rand::random::<u32>() as i64 % sides) + 1;
                }
            }
        } else {
            total += part.parse().unwrap_or(0);
        }
    }

    total
}
