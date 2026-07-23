# Conclave-rs Project Progress Report

**Generated:** 2026-07-23  
**Last Updated:** 2026-07-23  
**Status:** Active Development - Network Layer Complete, CLI Integration Phase

---

## Executive Summary

Building a decentralized P2P virtual tabletop for TTRPGs with plugin-based architecture, event-sourced sync between peers, and an optional relay node system. Currently in foundation phase: workspace structure, identity system, storage schema, and protocol definitions complete. Network layer being implemented with libp2p framework to support both direct P2P and the optional node system described in decentralization-ideas.md.

---

## Architecture Overview

### Core Design Decisions
- **Plugin System:** Dynamic libraries with shared core crate (<1% overhead)
- **Identity:** Ed25519 keypairs with 12-word BIP39 mnemonic recovery
- **Communication:** Hybrid event + RPC model
- **UI Framework:** Tauri + web tech (Obsidian-like layouts deferred from MVP)
- **Vector Search:** sqlite-vss for rule Q&A (MVP deferred)
- **Data Migration:** Explicit upgrade scripts on rule set changes
- **Security:** Code signing + user warnings for unverified plugins
- **Asset Transfer:** Chunked with resume capability, content-addressed by SHA-256 hash

### Networking & Node System
**Network Stack: libp2p Framework**
- Direct P2P connections via TCP/QUIC transports
- Built-in NAT traversal (hole punching) - no custom implementation needed
- Built-in relay protocol for remote nodes - maps directly to our node system
- Peer discovery via DHT and mDNS out of the box

**Optional Node System (from decentralization-ideas.md):**
- Relay traffic between peers that cannot connect directly
- Store campaign cache (event log subset, not full authority)
- Synchronize peers when they come online
- Cache assets for faster distribution
- Optional transcription service (voice → text)
- Optional AI features (summaries, NPC extraction, quest tracking)

**Critical Constraint:** The node does NOT own the campaign. It is simply another peer that usually stays online and provides availability improvements without becoming a central server or single point of failure.

---

## Implementation Status

### ✅ Completed Components

#### 1. Workspace Structure
```
crates/
├── core/           # Identity management, seed phrase generation
├── protocol/       # Event types, serde serialization
├── storage/        # SQLite schema per campaign
├── network/        # libp2p implementation (in progress)
├── plugin/         # Dynamic library loader with C ABI
├── app/            # CLI binary for MVP testing
└── node/           # Optional relay node (NOT YET CREATED - Phase 3+)
```

#### 2. Identity System (`crates/core`)
- Ed25519 keypair generation on first launch
- BIP39 mnemonic seed phrase (12 words) for recovery
- Display name and avatar storage
- Seed phrase export/import for testing
- **Tests:** All passing ✅

#### 3. Protocol Layer (`crates/protocol`)
```rust
enum Event {
    ChatMessage { author, content, timestamp },
    DiceRolled { expression, result },
    MemberJoined { player_id, display_name },
    CampaignCreated { dm_id, name },
}
```
- Serde serialization for all event types
- Signed event wrapper with Ed25519 signatures
- **Tests:** All passing ✅

#### 4. Storage Layer (`crates/storage`)
**SQLite Schema per Campaign:**
- `campaigns` - metadata (id, name, rule_set, dm_id)
- `members` - player permissions (dm, player, spectator)
- `events` - immutable append-only event log with signatures
- `player_data` - character sheets, notes
- `world_data` - DM-owned maps, NPCs, monsters
- `plugin_state` - per-plugin persistent data
- `assets` - content-addressed file storage (SHA-256 hashes)
- `sync_peers` - sync state tracking

#### 5. CLI Application (`crates/app`)
**Commands Implemented:**
- `init` - Generate identity + show seed phrase
- `identity` - Display current identity info
- `new-campaign <name>` - Create new campaign
- `list-campaigns` - Show all campaigns
- `roll <expression>` - Test dice roller plugin

#### 6. Plugin System (`crates/plugin`)
- Dynamic library loading via `libloading`
- TOML manifest parsing with version constraints
- C ABI for cross-language compatibility
- Basic RPC method routing

#### 7. Network Layer (`crates/network`) - **COMPLETE** ✅
- libp2p Swarm with TCP/QUIC transports
- mDNS peer discovery for LAN
- Identity integration (Ed25519 → PeerID)
- Event broadcast protocol (`/conclave/event/1.0.0`)
- Campaign event sync request/response
- CampaignDbHandle for serving incoming sync requests
- RPC system with `get_members`, `get_max_sequence`, `get_campaign_info` methods
- 9 passing integration tests

#### 8. CLI Integration (`crates/app`) - **COMPLETE** ✅
**Commands Implemented:**
- `init <name>` - Generate identity + show seed phrase
- `identity` - Display current identity info  
- `new-campaign <name> [--rule-set]` - Create new campaign, broadcasts CampaignCreated event
- `list-campaigns` - Show all campaigns with metadata (name, DM ID)
- `join-campaign <campaign_id> <peer_addr>` - Full peer connect + event sync + MemberJoined broadcast + plugin loading
- `peers` - List connected peers
- `connect <addr>` - Manual peer connection
- `listen --port 7777` - Start network listener with event callback processing
- `chat <campaign> <message>` - Send chat message (local storage)
- `roll <expression> [--campaign]` - Roll dice, record in campaign if specified
- `members <campaign_id>` - List campaign members with roles
- `leave-campaign <campaign_id>` - Broadcast MemberLeft event
- `transfer-dm <campaign_id> <target_player_id>` - Transfer DM authority
- `rpc-members <campaign_id> <peer_addr>` - Query remote peer for members via RPC
- `rpc-info <campaign_id> <peer_addr>` - Query remote peer for campaign metadata via RPC
- `load-plugin <campaign_id> <path>` - Load plugin with campaign context
- `unload-plugin <name>` - Unload loaded plugin
- `list-plugins` - Show all loaded plugins
- `status` - Comprehensive system state (identity, campaigns, plugins, network)

**Event Callback System:**
- Automatic member table updates for MemberJoined/MemberLeft events
- DmTransferred role updates
- Chat/Dice event display in listen mode

### ❌ Not Started

#### Phase 2+ Features (Deferred from MVP)
- Obsidian-like drag-and-drop UI layout system
- Character sheet plugins (D&D 5e, Pathfinder 2e)
- Rule set Q&A with sqlite-vss integration
- Voice chat (Opus codec) + transcription
- Map viewer plugin
- Combat tracker plugin
- P2P plugin distribution
- Code signing infrastructure

---

## Test Coverage

### Passing Tests (21 total)

**Identity & Protocol:**
1. **Identity Generation** - Key pair creation and seed phrase derivation
2. **Seed Phrase Recovery** - Import from mnemonic works correctly  
3. **Event Serialization** - All event types serialize/deserialize properly
4. **Campaign Creation** - SQLite database initialized with correct schema
5. **Plugin Manifest Parsing** - TOML manifests load with version validation
6. **Sync Message Protocol** - Handshake and event transfer messages work
7. **Sign/Verify Event Cycle** - Ed25519 signatures on events work correctly
8. **Tamper Detection** - Modified events fail signature verification

**Network Layer:**
9-17. **Integration Tests (9 tests)**:
   - Peer binding and address discovery
   - mDNS peer discovery
   - Manual peer connection
   - Event broadcast end-to-end
   - Campaign event sync request/response
   - MemberJoined event propagation
   - MemberLeft event propagation
   - Network command handling
   - Callback processing

**Storage:**
18-20. **Database Operations**: Event storage, retrieval, sequence tracking

**RPC System:**
21. **Campaign Info Query** - get_campaign_info RPC method works

---

## Known Issues & Risks

### Technical Risks

1. **Network Layer Migration** ⚠️ MEDIUM
   - Rewriting from QUIC to libp2p framework
   - Learning curve for libp2p API and concepts
   - **Mitigation:** Start with direct connections only, add relay support later

2. **Node System Design** ⚠️ LOW (future concern)
   - Node must not become single point of failure or campaign owner
   - Must handle partial data caching without breaking event sourcing
   - **Mitigation:** Treat node as just another peer with persistent connection

3. **Plugin Isolation** ⚠️ MEDIUM
   - Dynamic libraries share process memory (crash risk)
   - Symbol collisions possible with different dependency versions
   - **Mitigation:** Acceptable for MVP; process-based isolation for production

4. **Event Ordering Conflicts** ⚠️ LOW
   - Offline peers may create conflicting events
   - **Mitigation:** DM assigns sequence numbers; DM version always wins

5. **Data Migration at Scale** ⚠️ MEDIUM
   - Rule set upgrades require explicit migration scripts
   - Large campaigns may take time to migrate
   - **Mitigation:** Automatic backups before migration, progress indicators

### Open Questions

1. Should plugins communicate directly or only through core?
2. How to handle plugin updates mid-campaign? (lock vs upgrade)
3. Plugin discovery UI: registry browser vs simple file loading?
4. Code signing infrastructure for verified plugins?
5. **Node deployment model:** Self-hosted by DM? Community-run relays? Both?

---

## Development Environment

### Platforms
- **Primary:** Linux x86_64 (development machine)
- **Secondary:** macOS ARM64 (Apple Silicon, cross-platform testing)

### Toolchain
- Rust 1.75+ (stable)
- Cargo for dependency management
- SQLite 3.40+ for database layer

### Test Matrix
```
Linux ↔ Linux   ✅ Tested
macOS ↔ macOS   ⚠️ Not yet tested
Linux ↔ macOS   ❌ Not yet tested
Node relay      ❌ Not implemented (Phase 3+)
```

---

## Next Milestones

### MVP Definition of Done
- [x] Two instances connect on LAN via libp2p ✅
- [x] Create campaign on Peer A, join from Peer B ✅
- [ ] Send chat/dice events from A → B receives it (needs interactive mode)
- [x] Plugin loads and responds to RPC call ✅
- [x] Works offline, syncs when reconnected ✅
- [ ] Optional node can relay between peers (post-MVP)

### Immediate Next Steps
1. ✅ Network layer complete with libp2p TCP/QUIC transports
2. ✅ Event sync protocol implemented and tested
3. ⏳ Add interactive chat mode while listening (`conclave listen` + send commands)
4. ⏳ Wire up `chat` command to broadcast via network manager when peers connected
5. ⏳ End-to-end test: two CLI instances, full campaign workflow
6. ⏳ Update README with complete usage examples

---

## Timeline Estimate

| Phase | Duration | Status |
|-------|----------|--------|
| Foundation (workspace, identity, storage) | 2 weeks | ✅ Complete |
| Protocol & Plugin System | 1 week | ✅ Complete |
| Network Layer (libp2p) | 2-3 weeks | ⏳ In Progress |
| Sync Protocol | 1 week | ❌ Not Started |
| Integration Testing | 1 week | ❌ Not Started |

**Estimated MVP Completion:** 5-6 weeks from libp2p implementation start

---

## Dependencies

### Core Dependencies (in use)
- `tokio` - Async runtime
- `serde` + `serde_json` - Serialization
- `rusqlite` - SQLite database
- `ed25519-dalek` - Cryptography
- `bip39` - Mnemonic seed phrases
- `libloading` - Dynamic library loading
- `toml` - Manifest parsing
- `uuid` - Campaign IDs

### Pending Dependencies (network layer)
**libp2p Framework:**
- `libp2p-identity` - Peer ID management from Ed25519 keypairs
- `libp2p-tcp`, `libp2p-quic` - Transport protocols
- `libp2p-dns` - Service discovery (mDNS)
- `libp2p-relay` - Relay nodes for NAT traversal (Phase 3+)
- `libp2p-request-response` - RPC-style communication
- `libp2p-stream` - Custom event sync protocol

---

## Node System Architecture (from decentralization-ideas.md)

```
                  Desktop App
                       │
       ┌───────────────┼────────────────┐
       │               │                │
   UI Layer      Game Engine      Voice/Chat
       │               │                │
       └───────────────┼────────────────┘
                       │
                 Event System
                       │
           ┌───────────┴───────────┐
           │                       │
      SQLite Storage         Networking
                                   │
                ┌──────────────────┴─────────────────┐
                │                                    │
           LAN Peers                     Optional Node
                                              │
                                  Relay • Sync • Cache • AI
```

**Node Capabilities:**
- Relay traffic between peers that can't connect directly
- Store campaign event log cache (subset, not full authority)
- Synchronize peers when they come online
- Cache assets for faster distribution
- Optional transcription service (voice → text)
- Optional AI features (summaries, NPC extraction, quest tracking)

**Important:** The node does NOT own the campaign. It is simply another peer that usually stays online and provides availability improvements without becoming a central server.

---

## Contact & Documentation

- **Architecture:** `planning/architecture.md` (comprehensive, updated with libp2p decision)
- **Vision:** `planning/decentralization-ideas.md` (node system + full design)
- **Progress:** This file (`PROGRESS.md`)
- **Tasks:** `TODOS.md` (prioritized task list)
- **Code:** `crates/` directory structure

---

## Notes

This project prioritizes working P2P sync over UI polish for MVP. The Obsidian-like layout system, character sheets, and rule Q&A are deferred until after core connectivity is proven. Focus remains on event-sourced synchronization between peers with plugin extensibility and an optional relay node system.

**Key Decision:** Switched from raw QUIC (quinn) to libp2p framework for production-ready P2P infrastructure that maps directly to our requirements:
- Built-in NAT traversal vs custom implementation
- Built-in relay protocol for remote nodes
- Peer discovery via DHT + mDNS out of the box

**MVP Strategy:** Start with direct connections only using libp2p TCP/QUIC transports. Enable relays later when deploying public nodes - same codebase either way, no rewrite required.
