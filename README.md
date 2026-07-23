# Conclave-rs

A decentralized, peer-to-peer virtual tabletop for TTRPGs built in Rust.

## Features

- **No central server** - All peers sync campaign data via event log
- **Event-sourced architecture** - Every action is a signed, immutable event
- **Plugin system** - Campaign-specific dynamic libraries for extensibility
- **DM authority model** - DM controls world state, not a server
- **Libp2p networking** - P2P connections with NAT traversal support

## Quick Start

### 1. Build from source

```bash
cargo build --release
```

### 2. Create your identity (first run only)

```bash
./target/release/conclave init --name "Your Character Name"
```

Save the seed phrase shown - it's required to recover your identity!

### 3. Create a campaign (as DM)

```bash
./target/release/conclave new-campaign "My Campaign" --rule-set "pathfinder-2e@1.0.0"
# Output: Campaign created: <campaign-id>
```

### 4. Start listening for connections

```bash
./target/release/conclave listen --port 7777
```

### 5. Join a campaign (as player)

```bash
./target/release/conclave join-campaign <campaign-id> <dm-host:port>
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `init --name <name>` | Create new identity with display name |
| `identity` | Show current identity info |
| `new-campaign <name>` | Create a new campaign (DM only) |
| `list-campaigns` | List all local campaigns |
| `join-campaign <id> <peer>` | Join an existing campaign |
| `leave-campaign <id>` | Leave a campaign |
| `members <id>` | List campaign members and roles (local DB) |
| `transfer-dm <id> <player-id>` | Transfer DM authority |
| `chat <campaign> <message>` | Send a chat message |
| `roll <expression> --campaign <id>` | Roll dice (e.g., "2d20+5") |
| `load-plugin <campaign> <path>` | Load a plugin for campaign |
| `unload-plugin <name>` | Unload a loaded plugin |
| `list-plugins` | List all loaded plugins |
| `status` | Show current system status |
| `listen --port <port>` | Start network listener |
| `rpc-members <id> <peer>` | Query remote peer for campaign members |
| `rpc-info <id> <peer>` | Query remote peer for campaign metadata |

## Architecture

See [`planning/architecture.md`](planning/architecture.md) for detailed design decisions.

### Key Components

- **crates/protocol** - Event types and serialization
- **crates/core** - Identity management (Ed25519 + BIP39)
- **crates/storage** - SQLite persistence per campaign
- **crates/network** - Libp2p P2P networking layer
- **crates/plugin** - Dynamic plugin loading system
- **crates/node** - Relay node implementation (WIP)
- **crates/app** - CLI binary

### Event Types

- `CampaignCreated` - Campaign initialization
- `MemberJoined/Left` - Player membership changes
- `ChatMessage` - Text chat
- `DiceRolled` - Dice roll results with individual die faces
- `DmTransferred` - DM authority handoff
- `PluginLoaded` - Plugin activation
- Custom plugin events

## Testing

```bash
cargo test --all
cargo clippy --all-targets
```

## License

MIT
