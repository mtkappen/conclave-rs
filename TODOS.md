# Conclave-rs TODO List

**Last Updated:** 2026-07-23  
**Current Phase:** Network Layer (libp2p)

---

## Critical Blockers (Do First)

### [ ] 1. Set up libp2p dependencies
- Add to `crates/network/Cargo.toml`:
  - `libp2p` workspace crate with features: tcp, quic, dns, identify, ping, relay, request-response, mdns
  - `libp2p-identity` for PeerID from Ed25519 keypairs
  - `tokio-stream` for async stream handling

### [ ] 2. Rewrite network layer for libp2p
- Replace QUIC stub with actual libp2p implementation
- Create `NetworkManager` that:
  - Builds libp2p swarm from Ed25519 keypair (from `conclave-core`)
  - Listens on TCP/QUIC transports
  - Handles peer discovery via mDNS (LAN)
  - Manages connection lifecycle
- Create `PeerHandle` abstraction for sending/receiving events

### [ ] 3. Create node crate placeholder
- Add `crates/node/Cargo.toml` with basic structure
- Add to workspace `Cargo.toml` members
- Create stub `lib.rs` documenting node responsibilities:
  - Relay traffic between peers
  - Cache campaign events (not authority)
  - Sync offline peers when they reconnect

---

## MVP Core Tasks

### [ ] 4. Implement peer-to-peer event sync protocol
- Define libp2p protocol IDs for:
  - Handshake/identification
  - Event broadcast
  - Event request (pull missing events)
- Implement `EventCodec` for framing serialized events over streams

### [ ] 5. Integrate network layer with CLI
- Add `--port` flag to CLI for binding
- Add `connect <addr>` command to manually connect to peer
- Add `peers` command to list connected peers
- Add `broadcast <message>` command to test event sync

### [ ] 6. End-to-end integration tests
- Test two CLI instances on same machine (different ports)
- Verify event propagation: Peer A → Peer B
- Test offline scenario: disconnect, create events, reconnect, sync

---

## Post-MVP Features

### [ ] 7. Enable libp2p relay protocol
- Configure relay nodes in network config
- Implement NAT traversal for peers behind firewalls

### [ ] 8. Campaign synchronization logic
- Track last known event sequence per peer
- Request missing events on reconnect
- Handle concurrent event ordering (DM sequence numbers)

### [ ] 9. Asset transfer protocol
- Content-addressed chunking (SHA-256)
- Resume capability for large files
- Parallel chunk downloads

### [ ] 10. Full node implementation
- Campaign cache storage
- Peer sync on connect/disconnect events
- Optional: transcription service, AI features

---

## Deferred from MVP

- [ ] Obsidian-like UI layout system (Tauri)
- [ ] Character sheet plugins (D&D 5e, Pathfinder)
- [ ] Rule set Q&A with sqlite-vss
- [ ] Voice chat (Opus codec) + transcription
- [ ] Map viewer plugin
- [ ] Combat tracker plugin
- [ ] P2P plugin distribution
- [ ] Code signing infrastructure

---

## Notes

**Network Layer Strategy:** Start with direct connections only (TCP/QUIC). Enable relays later when deploying public nodes - same codebase either way.

**Testing Approach:** Use port 0 for dynamic port allocation in tests, manually specify ports for CLI testing on same machine.
