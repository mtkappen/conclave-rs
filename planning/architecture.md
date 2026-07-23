# Conclave-rs Architecture Decisions & Requirements

**Last Updated:** 2026-07-23

---

## Core Philosophy

A decentralized, peer-to-peer virtual tabletop for TTRPGs where:
- Every player runs the same application
- No permanent hosted server required
- Campaign data syncs via event log between peers
- DM is authority for world state, not a server
- Everything is plugin-based and campaign-specific

---

## 1. Plugin System

### Distribution Model
- **Primary:** Single hosted source (plugin registry)
- **Secondary:** P2P transfer when joining campaigns (player A has plugin B needs)
- **Format:** Dynamic libraries (.so/.dll/.dylib) compiled from Rust crates
- **Manifest:** TOML-based with semantic versioning per campaign

### Version Conflict Resolution
```
Rule Set (e.g., "pathfinder-2e v1.0")
    ↓ defines required versions
Plugin A v2.3.0
Plugin B v1.5.1  
Plugin C v3.0.0
    ↓
Campaign Manifest locks to these versions
    ↓
Players must match DM's plugin versions exactly
```

**Issue identified:** What if a player has an older version of a plugin installed globally?
- **Solution:** Campaign loads its specific plugin versions into isolated paths
- Only active campaign plugins are loaded at any time
- Plugin registry should support multiple versions of same plugin

### Plugin Lifecycle
1. `init()` - Register RPC endpoints and event subscriptions
2. `load_campaign(campaign_data)` - Initialize for specific campaign
3. Event handlers (async)
4. RPC method handlers (sync)
5. `unload_campaign()` - Cleanup before switching campaigns

### Communication Model: Hybrid Event + RPC

**Events (Pub/Sub)**
- ChatMessage, VoiceTranscript, DiceRolled
- TokenMoved, DamageDealt, SpellCast
- CampaignSyncStarted, PluginLoaded

**RPC (Synchronous Queries)**
- `get_character(player_id)` → CharacterData
- `roll_dice(expression)` → RollResult  
- `query_rule(topic)` → RuleReference
- `get_plugin_info(name)` → PluginMetadata

---

## 2. Layout System (Obsidian-like)

### Framework Decision: **Tauri + React/Vue**

**Why not egui?**
- Obsidian's layout system relies on complex drag-and-drop panels, tabs, and resizable split views
- egui is designed for immediate-mode UI (game-like), not document-style layouts
- Implementing Obsidian-style panel management in egui would require significant custom work
- Web tech has mature libraries for this exact use case (react-split-pane, react-draggable, etc.)

**Why Tauri?**
- Rust backend for game logic, networking, database
- Web frontend for UI (can look exactly like Obsidian)
- Much smaller binary than Electron
- Good Rust/JS interop via tauri-api

### Layout Storage Format
```json
{
  "version": "1.0",
  "layout": {
    "type": "h-split",
    "children": [
      {
        "type": "tab-group",
        "tabs": ["chat", "dice", "characters"],
        "active": "chat"
      },
      {
        "type": "v-split",
        "children": [
          { "type": "panel", "id": "rules-browser" },
          { "type": "panel", "id": "combat-tracker" }
        ]
      }
    ]
  }
}
```

**Question:** Layouts per-campaign or global?
- **Recommendation:** Both
  - Global default layout for new campaigns
  - Per-campaign overrides saved when user customizes

---

## 3. Identity & Recovery

### One Main Identity Across All Campaigns
- Single Ed25519 keypair generated on first launch
- Mnemonic seed phrase (12 words, BIP-39 standard) for recovery
- Display name and avatar stored locally (not cryptographic)

### Character Identities
```
Real Identity (your account)
    ├── Character 1: "Gandalf the Grey" (in-chat alias + character sheet)
    ├── Character 2: "Pippin Took" 
    └── OOC Mode (just your real identity)
```

- Switching between identities changes chat author name and avatar
- Character sheets are campaign-specific data owned by player
- Real identity proves you're authorized to access campaign

### Recovery Flow
1. First launch → generate keypair + show seed phrase
2. User backs up seed phrase (mandatory confirmation)
3. If device lost:
   - Install fresh
   - Restore from seed phrase
   - Rejoin campaigns you were in
   - Download campaign data + character sheets from peers

**Critical:** Seed phrase = full identity recovery. No central server to reset password.

---

## 4. Vector Database for Rule Q&A

### Qdrant Considerations

**Option A: Qdrant Embedded (rust version)**
- ✅ Runs locally, no external service needed
- ✅ Offline support works naturally
- ❌ Still early in development, may have bugs
- ❌ Memory usage could be high for large rule sets

**Option B: sqlite-vss (SQLite Vector Extension)**
- ✅ Single file database (fits campaign DB model)
- ✅ Already familiar to users of SQLite
- ❌ Less mature than Qdrant
- ❌ Fewer features (no filtering, scoring options limited)

**Option C: Local Qdrant Server Process**
- ✅ Full Qdrant feature set
- ✅ Can be bundled with app
- ❌ Extra process to manage
- ❌ More complex deployment

### Recommendation: **sqlite-vss for MVP, migrate to embedded Qdrant if needed**

Why?
- Campaign DB is already SQLite
- Rule embeddings are small (one rule set = few thousand vectors)
- Simpler architecture = fewer things to break
- Can upgrade later without data migration

### Embedding Strategy
- **Local computation:** User's machine generates embeddings when installing rule pack
- **Pre-computed option:** Hosted rule packs include embeddings (faster install)
- **Privacy:** Embeddings stay local, no data leaves user's machine

---

## 5. Campaign Structure & DM Control

### Campaign Creation Flow
```
DM selects: "Pathfinder 2e Rule Set v1.0"
    ↓
System loads rule set manifest:
  - Required plugins: character-sheet-pf2e, dice-roller, combat-tracker, spells-db
  - Recommended plugins: map-viewer, voice-chat, journal
  - Plugin versions locked by rule set
    ↓
DM configures campaign:
  - Campaign name, description
  - Optional custom rules (overrides)
  - Which recommended plugins to enable
    ↓
Campaign manifest created:
  {
    "id": "uuid",
    "rule_set": "pathfinder-2e@1.0.0",
    "required_plugins": [...],
    "optional_plugins": [...],
    "dm_customizations": {...}
  }
```

### Player Join Flow
```
Player requests to join campaign
    ↓
DM approves (or auto-approve enabled)
    ↓
System checks player's plugins:
  - Missing required? → Auto-download from registry or P2P from other players
  - Version mismatch? → Download correct version
  - All good? → Grant access
    ↓
Player downloads campaign data:
  - Event log (or since last sync point)
  - DM-owned content (maps, NPCs, monsters)
  - Plugin binaries for required plugins
```

### Issue Identified: Plugin Dependency Chain
```
Rule Set v1.0 requires:
  - Character Sheet v2.0
    - Depends on: Core Utils v1.5
  
Problem: What if another rule set needs Core Utils v2.0?

Solution: 
  - Campaign loads plugins in isolated paths
  - Each plugin bundles its dependencies OR
  - Shared "core" plugins can have multiple versions loaded
```

---

## 6. Database Schema (Per Campaign)

### Core Tables
```sql
-- Campaign metadata
campaigns (
  id TEXT PRIMARY KEY,      -- UUID
  name TEXT,
  rule_set TEXT,            -- e.g., "pathfinder-2e@1.0.0"
  created_at TIMESTAMP,
  dm_id TEXT                -- Ed25519 public key
)

-- Members and permissions
members (
  campaign_id TEXT,
  player_id TEXT,           -- Ed25519 public key
  role TEXT,                -- 'dm', 'player', 'spectator'
  joined_at TIMESTAMP,
  PRIMARY KEY (campaign_id, player_id)
)

-- Event log (immutable, append-only)
events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  campaign_id TEXT,
  event_type TEXT,          -- 'ChatMessage', 'DiceRolled', etc.
  author_id TEXT,           -- Who created this event
  timestamp TIMESTAMP,
  sequence_number INTEGER,  -- DM-assigned, monotonically increasing
  signature TEXT,           -- Cryptographic signature for authenticity
  payload JSON              -- Event-specific data
)

-- Player-owned data (characters, notes, etc.)
player_data (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  campaign_id TEXT,
  owner_id TEXT,            -- Player who owns this
  data_type TEXT,           -- 'character', 'notes', 'inventory'
  content JSON,
  updated_at TIMESTAMP
)

-- DM-owned data (maps, NPCs, monsters, etc.)
world_data (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  campaign_id TEXT,
  owner_id TEXT,            -- Should always be dm_id
  data_type TEXT,           -- 'map', 'npc', 'monster', 'quest'
  content JSON,
  updated_at TIMESTAMP
)

-- Plugin state (per-plugin, per-campaign)
plugin_state (
  plugin_name TEXT,
  campaign_id TEXT,
  version TEXT,
  state JSON,               -- Plugin-specific persistent data
  PRIMARY KEY (plugin_name, campaign_id)
)

-- Asset storage (content-addressed by hash)
assets (
  hash TEXT PRIMARY KEY,    -- SHA-256 of file content
  filename TEXT,
  mime_type TEXT,
  size INTEGER,
  uploaded_by TEXT,
  uploaded_at TIMESTAMP
)

-- Sync state (for each peer we've synced with)
sync_peers (
  player_id TEXT PRIMARY KEY,
  last_sync_event_id INTEGER,  -- Last event ID we exchanged
  last_sync_time TIMESTAMP
)
```

### Vector Search Table (sqlite-vss)
```sql
-- Rule embeddings for Q&A
rule_embeddings (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  rule_set TEXT,            -- e.g., "pathfinder-2e"
  category TEXT,            -- "spells", "combat", "skills"
  title TEXT,
  content TEXT,             -- Full rule text
  embedding BLOB,           -- Vector data
  source_file TEXT          -- Original markdown file path
)

-- Enable vector search index
CREATE VIRTUAL TABLE rule_embeddings USING vss0(
  embedding(1536),
  distance_metric='cosine'
);
```

---

## 7. Networking & Sync

### Connection Types
1. **LAN:** QUIC with automatic discovery (mDNS or similar)
2. **Internet Direct:** QUIC with public IP/hostname
3. **Relay Node:** When direct connection not possible (NAT traversal failure)

### Sync Protocol
```
Peer A connects to Peer B
    ↓
Exchange sync state:
  A: "I have events 1-100, know of peers [X, Y]"
  B: "I have events 1-95, know of peers [A, Z]"
    ↓
Identify missing events:
  A needs: none (has everything B has)
  B needs: 96-100
    ↓
Transfer missing events (encrypted, signed)
    ↓
Update sync_peers table with new last_sync_event_id
```

### Event Ordering & Conflict Resolution
- **DM assigns sequence numbers** to world-affecting events
- **Players assign local timestamps** to personal events (chat, notes)
- **Conflict resolution:** DM's version always wins for shared state
- **Append-only logs:** Never edit history, only add new events

---

## 8. Security Model

### Cryptography
- **Identity:** Ed25519 keypair (seed phrase → private key)
- **Event signing:** All events signed by author's private key
- **Transport:** TLS 1.3 / QUIC encryption
- **Campaign encryption:** Optional AES-GCM with shared campaign key

### Permissions Matrix
| Action | Player | DM | Spectator |
|--------|--------|----|-----------|
| Chat | ✅ | ✅ | ✅ |
| Voice | ✅ | ✅ | ✅ (if enabled) |
| Edit character sheet | ✅ (own only) | ✅ (all) | ❌ |
| View maps | ✅ | ✅ | ✅ (if DM allows) |
| Reveal fog of war | ❌ | ✅ | ❌ |
| Modify combat state | ❌ | ✅ | ❌ |
| Invite members | ❌ | ✅ | ❌ |
| Transfer DM role | ❌ | ✅ | ❌ |

---

## 9. Open Questions & Risks

### Technical Risks

1. **Dynamic library versioning:** How to safely load different plugin versions without conflicts?

   **Reality check:** True isolation with dynamic libraries is difficult:
   - `dlopen` loads into process address space - symbols can collide
   - Rust's standard library may be linked multiple times (bloat)
   - Platform-specific solutions needed (Linux: `RTLD_DEEPBIND`, macOS: symbol scoping)

   **Recommended approach:** Use a **plugin ABI with explicit namespacing**
   ```rust
   // Each plugin exports C-compatible functions with unique prefix
   #[no_mangle]
   pub extern "C" fn pf2e_character_sheet_init() { ... }
   
   #[no_mangle]  
   pub extern "C" fn pf2e_character_sheet_get_character(id: i32) -> *const u8 { ... }
   ```

   **Performance cost:** Minimal (~1-2% overhead from function indirection). Real memory bloat comes from duplicate stdlib linking - estimate 50-100MB per plugin if fully isolated.

   **Alternative if speed is critical:** Single shared "core" crate loaded once, plugins link against it at compile time. Tradeoff: all campaigns share core version, but plugins can still be versioned independently.

2. **P2P plugin distribution:** What if no peer has the required plugin?
   - *Solution:* Fallback to hosted registry when internet available
   - *Join constraint:* Player must have at least one online peer (DM or other player) to join campaign and sync plugins/data

3. **Vector DB at scale:** If rule sets grow to 100k+ embeddings, will sqlite-vss perform well?
   - *Mitigation:* Start with sqlite-vss, benchmark, migrate to embedded Qdrant if needed

4. **Offline-first conflicts:** What happens when two DMs make conflicting changes while offline?
   - *Solution:* Only one DM per campaign at a time (DM role transfer is explicit event)

5. **Rule set upgrades and data migration:**
   - *Decision:* Force explicit migration when upgrading rule sets
   - Users likely run multiple campaigns with same rule set → one-time migration benefits all
   - Migration scripts versioned alongside rule sets
   - Backward compatibility optional (prefer breaking changes with clear migration path)

### Design Questions Still Open
1. Should plugins be able to communicate directly with each other, or only through core?
2. How do we handle plugin updates mid-campaign? (Lock versions vs allow upgrades)
3. What's the plugin discovery UI? (Registry browser, search, ratings?)
4. Do we need a "plugin marketplace" concept, or just a simple registry?
5. How do we verify plugin authenticity? (Code signing with developer keys?)

---

## 10. Implementation Phases

### Phase 1: Foundation (Weeks 1-4)
- [ ] Tauri workspace setup
- [ ] Basic UI shell with panel system
- [ ] Identity generation + seed phrase backup flow
- [ ] SQLite database layer
- [ ] Plugin loader (dynamic libraries, TOML manifests)

### Phase 2: Campaign Core (Weeks 5-8)
- [ ] Campaign creation/management
- [ ] Event system (types, storage, retrieval)
- [ ] Basic plugin API (RPC + events)
- [ ] Character sheet plugin (D&D 5e example)
- [ ] Dice roller plugin

### Phase 3: Networking (Weeks 9-12)
- [ ] QUIC-based peer connections
- [ ] LAN discovery
- [ ] Event synchronization protocol
- [ ] Conflict resolution logic
- [ ] Relay node implementation

### Phase 4: Rule Sets (Weeks 13-16)
- [ ] Markdown parser for existing rule files
- [ ] sqlite-vss integration
- [ ] Embedding generation pipeline
- [ ] Rule Q&A plugin
- [ ] Pathfinder 2e rule pack

### Phase 5: Polish & Features (Weeks 17-20)
- [ ] Voice chat (Opus codec)
- [ ] Local transcription (optional)
- [ ] Map viewer plugin
- [ ] Combat tracker plugin
- [ ] Asset synchronization
- [ ] P2P plugin distribution

### Phase 6: Beta & Feedback (Weeks 21+)
- [ ] Security audit
- [ ] Performance optimization
- [ ] User testing
- [ ] Plugin ecosystem growth

---

## Final Architecture Decisions (2026-07-23)

### Plugin Isolation Strategy

**Question:** Can we achieve isolated environments with dynamic libraries?

**Answer:** Partially, with tradeoffs:

| Approach | Isolation | Performance | Complexity |
|----------|-----------|-------------|------------|
| True isolation (separate processes) | ✅ Complete | ❌ IPC overhead (~10-20%) | ❌ High |
| Dynamic libs + symbol namespacing | ⚠️ Partial | ✅ Minimal (~1-2%) | ⚠️ Medium |
| Shared core crate + versioned plugins | ❌ None (shared memory) | ✅ Best (<1%) | ✅ Low |

**Decision:** **Shared core crate approach** for MVP:
- Core functionality (database, networking, event system) compiled once into main app
- Plugins are dynamic libs that link against core at runtime via C ABI
- Each campaign specifies which core version it needs
- Memory efficient, fast, but plugins share core state

**If true isolation needed later:** Migrate to process-based plugins with IPC. Accept performance cost for safety.

### Data Migration Strategy

**Decision:** Force explicit migration on rule set upgrades

```
User clicks "Upgrade Pathfinder 2e v1.0 → v1.1"
    ↓
System shows migration plan:
  - Schema changes required
  - Plugin version updates  
  - Estimated time: 30 seconds
    ↓
User confirms
    ↓
Migration runs on ALL campaigns using this rule set
  - Backup created automatically
  - Each campaign upgraded sequentially
    ↓
Done. All campaigns now on v1.1
```

**Why this works:**
- Users likely have multiple campaigns with same rule set → one upgrade action fixes all
- Explicit migration prevents silent data corruption
- Backups allow rollback if something breaks

### P2P Join Flow

**Clarified flow:**
```
Player A wants to join Campaign X
    ↓
Checks: Do I have required plugins?
  - No? → Pull from online peers (DM or other players)
  - Still missing? → Fallback to hosted registry (requires internet)
    ↓
Checks: Is at least one peer online?
  - Yes (DM or player B): Request join, sync data
  - No: Queue join request, notify when someone comes online
```

**Key constraint:** Cannot join campaign without at least one online peer to authorize and sync from. This is acceptable - DM should be available for new players to join.

---

## Notes from Architecture Discussion

**Date:** 2026-07-23

**Decisions Made:**
- Dynamic libraries with shared core crate (best performance, accept crash risk)
- Hybrid event+RPC communication model
- TOML manifests with semantic versioning per campaign
- Mnemonic seed phrase (12 words) for identity recovery
- One main identity across all campaigns, character aliases per campaign
- Tauri + web tech for Obsidian-like UI
- sqlite-vss for vector search (MVP), migrate to embedded Qdrant if needed
- Force explicit data migration on rule set upgrades
- P2P plugin distribution with hosted registry fallback

**Key Insight:** 
The rule set acts as a "template" that defines required plugin versions. When DM creates campaign with "Pathfinder 2e v1.0", all plugins are locked to versions specified by that rule set manifest. Players must match these versions exactly to join.

**Risk to Monitor:**
Plugin dependency conflicts when different rule sets need different versions of shared utilities. Solution: Shared core crate approach means all campaigns using same core version share utilities, but plugin-specific dependencies can still be namespaced.

---

## Additional Decisions (2026-07-23)

### Plugin Security Model

**Hybrid trust model:**
- **Verified plugins:** Code-signed with developer keys, marked as "trusted" in UI
- **Unverified plugins:** Users can install anything, but get clear warnings
- **Campaign warning:** If DM uses unverified plugins, players see: *"This campaign uses plugins from unverified sources. Proceed at your own risk."*

**Implementation:**
```toml
# Plugin manifest
[plugin]
name = "pf2e-character-sheet"
version = "1.0.0"
author = "Pathfinder Official"
signature = "ed25519:abc123..."  # Code signature

[trust]
verified = true
trusted_developer = true
```

UI shows trust badges: ✅ Verified, ⚠️ Unverified, ❌ Unknown author

### Event Schema Evolution

**Decision:** Migration scripts on upgrade

```rust
// Events store schema version
struct Event {
    id: u64,
    schema_version: u32,  // e.g., 1, 2, 3
    event_type: String,
    payload: serde_json::Value,
}

// Migration registry
fn migrate_event(event: Event, target_version: u32) -> Event {
    match (event.schema_version, target_version) {
        (1, 2) => apply_migration_v1_to_v2(event),
        (2, 3) => apply_migration_v2_to_v3(event),
        _ => event,
    }
}
```

Migration scripts versioned alongside rule sets. Run during campaign upgrade.

### Asset Transfer Protocol

**Chunked transfer with resume:**
- Files split into 1MB chunks
- Each chunk has SHA-256 hash for integrity
- Transfer can pause/resume (useful for unstable connections)
- Parallel chunk downloads from multiple peers if available

```rust
struct AssetTransfer {
    asset_hash: String,
    total_size: u64,
    chunk_size: u64 = 1024 * 1024,  // 1MB
    completed_chunks: Vec<u32>,      // Which chunks we have
}

// Peer A requests missing chunks from Peer B
// "I need chunks 5-12 of asset abc123"
```

### MVP Scope: Basic P2P Sync Only

**First working prototype priorities:**
1. Two peers can discover each other on LAN
2. Establish QUIC connection
3. Create campaign, sync events between peers
4. Basic plugin loading (even if just dummy plugins)
5. Command-line or minimal UI (not Obsidian-like yet)

**Defer until after MVP:**
- Drag-and-drop panel layout system
- Character sheet plugins
- Rule set Q&A with sqlite-vss
- Voice chat
- Full Tauri UI polish

---

## Final Checklist Before Coding

### Architecture Complete ✅
- [x] Plugin system design (shared core crate, dynamic libs)
- [x] Communication model (hybrid event + RPC)
- [x] Identity & recovery (mnemonic seed phrase)
- [x] Campaign database schema (SQLite per campaign)
- [x] Vector search strategy (sqlite-vss for MVP)
- [x] Versioning & migration (explicit upgrades)
- [x] Security model (code signing + user warnings)
- [x] Asset transfer (chunked with resume)

### Still Open (Decide During Implementation)
- [ ] Exact plugin ABI function signatures
- [ ] Event type catalog (complete list for v1)
- [ ] QUIC library choice (quinn vs tokio-quic)
- [ ] Tauri commands for plugin ↔ core communication
- [ ] Test strategy for P2P sync scenarios

### MVP Definition of Done
- [ ] Two instances can connect on LAN
- [ ] Create campaign on Peer A, join from Peer B
- [ ] Send event (chat message) from A → B receives it
- [ ] Plugin loads and responds to RPC call
- [ ] Works offline, syncs when reconnected

---

## Repository Structure (Final)

```
conclave-rs/
├── Cargo.toml                    # Workspace root
├── planning/
│   ├── architecture.md           # This file
│   ├── decentralization-ideas.md # Original vision doc
│   └── plans.md                  # TODO: Move MVP tasks here
├── rules/                        # Rule set markdown files
│   ├── dnd-5e/
│   ├── pathfinder-2e/
│   ├── coc-d20/
│   └── lancer/
├── src/                          # Core application (temporary)
│   └── main.rs                   # Will move to crates/
├── crates/                       # Workspace crates (TODO)
│   ├── core/                     # Game engine, event system
│   ├── protocol/                 # Event definitions, serialization
│   ├── storage/                  # SQLite layer, migrations
│   ├── network/                  # QUIC, sync protocol
│   ├── plugin/                   # Plugin loader, ABI
│   └── app/                      # Tauri desktop app
└── plugins/                      # Example plugins (TODO)
    ├── dice-roller/
    ├── character-sheet-dnd5e/
    └── rule-qna/
```

---

## MVP Requirements (Basic P2P Sync)

### Definition of Done
- [ ] Two instances connect on LAN (Linux ↔ macOS tested)
- [ ] Create campaign on Peer A, join from Peer B
- [ ] Send event (chat message) from A → B receives it
- [ ] Plugin loads and responds to RPC call
- [ ] Works offline, syncs when reconnected

### Core Components Needed

#### 1. Workspace Structure (`crates/`)
```
crates/
├── core/           # Event system, campaign state machine
├── protocol/       # Event definitions, serialization (serde)
├── storage/        # SQLite layer, migrations, schema
├── network/        # QUIC connections, LAN discovery
├── plugin/         # Plugin loader, ABI traits, manifest parsing
└── app/            # Minimal CLI or Tauri shell
```

#### 2. Identity System
- Ed25519 keypair generation on first run
- Seed phrase derivation (BIP-39)
- Display name/avatar storage
- Export/import identity for recovery testing

#### 3. Campaign Model
- UUID generation
- SQLite database per campaign
- Basic schema: campaigns, members, events tables
- Campaign creation/joining flow

#### 4. Event System
```rust
enum Event {
    ChatMessage { author, content, timestamp },
    DiceRolled { expression, result },
    MemberJoined { player_id, display_name },
    CampaignCreated { dm_id, name },
}
```
- Immutable events with signatures
- Append-only storage
- Query by campaign ID

#### 5. Plugin System (Minimal)
```toml
# plugin.toml manifest
[plugin]
name = "test-plugin"
version = "0.1.0"
core_version = "0.1.0"

[capabilities]
rpc = true
events = ["ChatMessage", "DiceRolled"]
```

```rust
// Plugin ABI (C-compatible)
#[repr(C)]
pub struct PluginInfo {
    name: *const c_char,
    version: *const c_char,
}

#[no_mangle]
pub extern "C" fn plugin_info() -> PluginInfo { ... }

#[no_mangle]
pub extern "C" fn plugin_init(core_api: CoreApi) -> i32 { ... }
```

- Dynamic library loading (`dlopen`/`dlsym`)
- TOML manifest parsing
- Basic RPC call routing

#### 6. Networking (QUIC)
- Peer-to-peer connection establishment
- LAN discovery (mDNS or broadcast)
- Connection pooling (multiple peers)
- Reconnection logic

#### 7. Sync Protocol
```
Peer A ↔ Peer B handshake:
1. Exchange peer IDs and campaign IDs
2. Compare last known event sequence numbers
3. Transfer missing events (chunked if large)
4. Update sync state
5. Close or keep alive for future syncs
```

#### 8. Minimal UI/CLI
**Option A: CLI (faster)**
- `conclave new-campaign --name "My Game"`
- `conclave join <campaign_id>`
- `conclave chat "Hello world"`
- `conclave list-peers`

**Option B: Basic Tauri Shell**
- Simple window with chat input and message log
- Connect/disconnect button
- Campaign ID display

#### 9. Testing Infrastructure
- Integration tests (two processes, verify sync)
- Cross-platform test matrix (Linux ↔ macOS)
- Offline/online scenario testing

### Dependencies to Select

| Purpose | Options | Recommendation |
|---------|---------|----------------|
| QUIC | `quinn`, `tokio-quic` | `quinn` (active, good docs) |
| SQLite | `rusqlite`, `sqlx` | `rusqlite` + migrations crate |
| Serialization | `serde`, `bincode` | `serde` + `serde_json` |
| Plugin loading | `libloading` | `libloading` (cross-platform dlopen) |
| TOML parsing | `toml` | `toml` crate |
| UUIDs | `uuid` | `uuid` with serde support |
| Crypto | `ed25519-dalek`, `ring` | `ed25519-dalek` (simple API) |
| BIP-39 | `bip39` | `bip39` crate |
| mDNS | `if-watch`, `mdns-sd` | `mdns-sd` (bonjour/avahi) |

### What's NOT in MVP

- [ ] Obsidian-like drag-and-drop UI (Tauri polish later)
- [ ] Character sheet plugins (dummy plugin only)
- [ ] Rule set Q&A / sqlite-vss (Phase 4)
- [ ] Voice chat (Phase 5)
- [ ] Asset transfer (chunked file sync - Phase 5)
- [ ] Code signing infrastructure (add before public release)
- [ ] Plugin registry website (manual .so/.dylib files for now)

### MVP Timeline Estimate

| Week | Focus | Deliverable |
|------|-------|-------------|
| 1 | Workspace + Identity | Crates set up, keypair generation works |
| 2 | Storage + Events | SQLite schema, event storage/retrieval |
| 3 | Plugin System | Load dynamic lib, call RPC method |
| 4 | Networking | Two peers connect via QUIC |
| 5 | Sync Protocol | Events transfer between peers |
| 6 | Integration | CLI/Tauri shell, end-to-end testing |

**MVP complete:** Week 6 (if focused, parallel work possible)

### Immediate Next Steps After Break

1. Set up Cargo workspace with crates layout
2. Create `protocol` crate with basic Event enum
3. Implement identity generation in `core` crate
4. Write first integration test skeleton

---

## Final Architecture Decisions (2026-07-23)

---

## Development & Test Environment

### Hardware
- **Linux PC** (x86_64) - Primary development machine
- **Apple Silicon Mac Pro 2020** (aarch64-darwin) - Cross-platform testing

### Platform-Specific Considerations

| Component | Linux (.so) | macOS (.dylib) | Notes |
|-----------|-------------|----------------|-------|
| Plugin loading | `dlopen`/`dlsym` | `dlopen`/`dlsym` | Same API, different file extensions |
| Symbol visibility | `-fvisibility=hidden` | `-exported_symbols_list` | Need to control exported symbols |
| Dynamic linking | Standard | SIP restrictions | macOS may require notarization for distribution |
| QUIC networking | Works | Works | Both support UDP/TLS 1.3 |
| SQLite | Built-in | Built-in | No platform differences |

### Cross-Compilation Strategy

```toml
# .cargo/config.toml
[build]
target = "x86_64-unknown-linux-gnu"  # Default for Linux dev

[target.aarch64-apple-darwin]
linker = "aarch64-apple-darwin21.3-clang"
rustflags = ["-C", "link-arg=-mmacosx-version-min=11.0"]
```

**Development workflow:**
- Build and test on Linux first (faster compile cycles)
- Cross-compile to macOS or build natively on Mac for platform testing
- Test P2P sync between Linux ↔ macOS peers early and often

### Platform-Specific Code

```rust
#[cfg(target_os = "linux")]
mod plugin_loader_linux {
    // Uses lib*.so files
}

#[cfg(target_os = "macos")]
mod plugin_loader_macos {
    // Uses *.dylib files
}
```

**Important:** Test on both platforms before any release. macOS has stricter security (Gatekeeper, notarization) that may affect plugin loading from non-App Store sources.
