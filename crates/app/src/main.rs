//! Conclave desktop application - MVP CLI

use clap::{Parser, Subcommand};
use conclave_core::Identity;
use conclave_network::NetworkManager;
use conclave_storage::open_campaign_db;

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
            println!("Joining campaign {} via {}...", campaign_id, peer_addr);
            
            let id_path = data_dir.join("identity.json");
            if !id_path.exists() {
                println!("Error: No identity found. Run 'conclave init' first.");
                return;
            }

            // TODO: Load identity, create network manager, connect to peer
            println!("TODO: Connect to peer at {} and sync campaign {}", peer_addr, campaign_id);
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
