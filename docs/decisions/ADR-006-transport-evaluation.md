# ADR-006: Transport Evaluation

**Status:** Accepted
**Date:** 2026-07-06

## Context

The communication protocol document lists several possible transports (WebSocket, TCP, USB/ADB, QUIC, BLE) and formats (Protocol Buffers, JSON, FlatBuffers, CBOR). This ambiguity blocks implementation — mobile and desktop cannot begin coding until the transport is chosen.

Rather than making an architectural guess, we validated candidates with a real prototype (see `spikes/ep-001-discovery-pairing/`).

## Decision

Adopt **Wi-Fi (WebSocket)** as the primary command transport, with **mDNS** for local discovery. **BLE** is deferred for a future milestone as an optional presence/discovery accelerator — it will never carry command payloads.

### Primary Transport: WebSocket over Wi-Fi

| Criterion | Verdict |
|-----------|---------|
| Android support | ✅ Native (OkHttp) |
| Windows support | ✅ Native (tokio-tungstenite) |
| Bidirectional | ✅ Full-duplex |
| Encryption | ✅ Noise protocol (Phase 3) or WSS |
| NAT/firewall | mDNS for LAN; manual IP for cross-subnet fallback |
| Latency | < 10 ms on same LAN (to be confirmed in hardware testing) |
| Throughput | Sufficient for sub-1 KB command messages |
| Debugging | ✅ Inspectable with Wireshark, browser DevTools, or custom tools |

### Provider Architecture: Presence vs Discovery

The discovery subsystem is split into two distinct abstractions with different responsibilities:

**Discovery Providers** resolve a desktop identity into a connection endpoint. Every provider returns a `DiscoveredDevice` with a valid `host:port` that the WebSocket transport can connect to. Current and future providers:

| Provider | Returns Endpoint? | Status |
|----------|:-:|--------|
| mDNS | ✅ Yes | Implemented in EP-001 |
| Cached hostname | ✅ Yes | Defined interface |
| Cached IP | ✅ Yes | Defined interface |
| Manual entry | ✅ Yes | Defined interface |
| Enterprise DNS | ✅ Yes | Future |
| BLE | ❌ No | See below |

**Presence Providers** answer only "is this desktop nearby?" They emit proximity events (`deviceId`, RSSI, timestamp) — they never return connection data. BLE is the primary candidate:

- Presence detection (phone arrived / left)
- Proximity triggers
- Wake-up signal to accelerate discovery

Presence providers are **not implemented in EP-001**. When BLE arrives in a future milestone, a `ConnectionManager` will sit between presence and discovery:

```
PresenceProvider → ConnectionManager → DiscoveryManager → TransportManager
```

`ConnectionManager` applies connection policy (debouncing, reconnection, Wi-Fi availability, user preferences, battery saver, multiple desktops) before invoking discovery. This keeps presence providers stateless and discovery providers focused on endpoint resolution.

No transport code depends on the presence layer. No presence provider fabricates endpoint data.

### Message Format: JSON (spike), TBD (production)

JSON is confirmed sufficient for the spike. Production format deferred to ADR-008 (follow-up after spike data collection).

### Rejected Candidates

| Transport | Reason |
|-----------|--------|
| **BLE** | Too low throughput, platform-specific APIs, pairing complexity, poor fit for structured command messages. Relegated to a future `PresenceProvider` (proximity events only — never returns endpoints). |
| **Raw TCP** | Requires manual framing and encryption. WebSocket provides framing, close handshake, and subprotocol negotiation natively. |
| **QUIC** | Too immature across both platforms for v0.1. Revisit for v0.5+ if latency becomes a bottleneck. |
| **USB/ADB** | Defeats the local-first/mobile-first vision. Kept as a debugging fallback only. |
| **gRPC** | Heavy dependency for sub-1 KB messages. Half-duplex streaming complicates the bidirectional protocol. |

## Rationale

- Wi-Fi provides low latency (single-digit ms), high bandwidth, full-duplex communication, and works naturally with a phone that moves around the room.
- mDNS is zero-configuration on consumer LANs — no server, no DNS setup.
- BLE presence is a clean separation: "phone is here" is a boolean signal, not a command channel.
- WebSocket adds framing and close semantics on top of TCP without needing to reinvent them.
- JSON is the fastest path to a working prototype; schema enforcement can be added later.

## Consequences

- EP-001 Phase 1 proceeds with WebSocket over Wi-Fi + mDNS.
- Discovery layer is abstracted as `DiscoveryProvider` (endpoint resolution) + `DiscoveryManager` (orchestrator) on Android; `AdvertisementProvider` (desktop-side advertisement) on desktop.
- Presence layer (`PresenceProvider`) is a separate concept — it emits proximity events only, never returns endpoints. **Not implemented in EP-001.** Defined as interface only; no implementation until BLE is added.
- When BLE arrives, a `ConnectionManager` will sit between `PresenceProvider` and `DiscoveryManager` to handle connection policy (debouncing, reconnection, Wi-Fi state, user prefs). Presence providers remain stateless.
- BLE design and implementation is deferred to a dedicated spike (EP-008 or later).
- ADR-008 will formalize the production message format after spike evidence is collected.
- The protocol document in `docs/architecture/02-protocol.md` should be updated to reflect this decision.
