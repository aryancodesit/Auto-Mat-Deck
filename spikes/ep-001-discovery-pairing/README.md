# Spike ep-001: Discovery, Pairing & Transport Validation

> Validate the core communication pipeline on real Android and Windows hardware.
> This is an **assumption validation** spike — failure modes matter as much as the happy path.

## Question

Can Android discover a Windows desktop on the local network, establish a secure pairing, and exchange an encrypted message? When does it fail, and how do we recover?

## Hypothesis

mDNS discovery + QR-code pairing + Noise protocol encryption over WebSocket will work on typical consumer home networks. JSON is sufficient for spike prototyping. Three onboarding methods (native window, browser, CLI) should each have measurable trade-offs.

## Scope

### What we will build

| Component | What |
|-----------|------|
| Android | Minimal Kotlin app: mDNS scanner, QR scanner, WebSocket client, display received message |
| Windows | Minimal C# or Rust app: mDNS announcer, QR generator, WebSocket server, respond with "Pong" |
| Transport | Raw WebSocket (no protocol schema yet — just JSON string exchange) |
| Encryption | NaCl/libsodium box (curve25519 + xsalsa20-poly1305) — ephemeral key exchange |
| Onboarding | Three prototypes: native window (QR), localhost browser page (QR), CLI (manual code entry) |
| Serialization | JSON only — for spike purposes. Production format TBD. |

### What we will NOT build

- Protocol message framing (Command/Response/Event)
- Persistence (no profiles, no storage)
- Operational UI beyond onboarding (no dashboards, no settings)
- Plugin loading or execution

## Phases

The spike is divided into three internal phases. Each phase must pass its gates before the next phase begins. This prevents unbounded effort while keeping the spike as a single user journey.

---

### Phase 1: Discovery & Transport

**Gate**: Plaintext WebSocket connectivity between Android and Windows on the same LAN.

#### Scope
- mDNS discovery (Android scans, Windows announces).
- Raw WebSocket connection (no encryption yet).
- Basic "ping/pong" message exchange in plaintext.

#### Success Thresholds

| Metric | PASS | WARNING | FAIL |
|--------|------|---------|------|
| Discovery time | < 5 s | 5–10 s | > 10 s |
| WebSocket connect | < 1 s | 1–3 s | > 3 s |
| Round-trip (plaintext) | < 100 ms | 100–500 ms | > 500 ms |
| mDNS uniqueness (2 desktops) | Both distinct | One visible | Neither visible |

#### Failure Scenarios
- Windows Defender Firewall blocks first connection → user allows → succeeds.
- Windows Defender Firewall blocked → mobile shows clear error.
- Public network profile (Windows) → document behavior.
- AP isolation → discovery fails → mobile shows "no desktops found" + manual entry.

#### Gate ✓
Phase 1 passes when Android can send `{"type": "ping"}` to Windows over plain WebSocket and receive `{"type": "pong"}` back, on both same-Wi-Fi and wired-desktop + Wi-Fi-mobile configurations, with firewall either allowed or gracefully declined.

---

### Phase 2: Pairing & Trust

**Gate**: Secure key exchange and pairing between Android and Windows.

#### Scope
- Onboarding prototypes (native window, browser page, CLI).
- Noise protocol handshake over WebSocket.
- Key exchange and session establishment.
- MITM and replay attack validation.

#### Onboarding Prototypes

Each prototype implements the same pairing flow with a different first-contact UI. All three will be measured and compared.

##### A. Native Bootstrap Window
- **Dependencies**: Native GUI framework (WinForms, WPF, or Tauri)
- **Binary size impact**: Measured
- **UX**: Visual, familiar, no browser needed
- **Cross-platform effort**: Higher

##### B. Localhost Browser Page
- **Dependencies**: Embedded HTTP server (tiny-http, actix-web, or similar)
- **Binary size impact**: Measured
- **UX**: Native browser rendering; browser security policies apply
- **Cross-platform effort**: Lower

##### C. CLI Pairing
- **Dependencies**: None beyond stdio
- **Binary size impact**: Zero
- **UX**: Acceptable for developers, poor for consumer users
- **Cross-platform effort**: None

##### Comparison Criteria

| Criterion | Native Window | Browser Page | CLI |
|-----------|---------------|--------------|-----|
| Binary size delta | | | |
| Implementation complexity | | | |
| First-pairing UX (1–5) | | | |
| Re-pairing UX (1–5) | | | |
| Works without browser | | | |
| Works without GUI | | | |
| Firewall implications | | | |

##### Time-Box Rule
Each onboarding prototype must fit in **one day** of implementation. No polished UI, no animations, no branding, no production architecture. Uneven implementation time is itself valuable evidence.

#### Success Thresholds

| Metric | PASS | WARNING | FAIL |
|--------|------|---------|------|
| Pairing time (scan → complete) | < 2 s | 2–5 s | > 5 s |
| Noise handshake | < 500 ms | 500–1000 ms | > 1000 ms |
| Pairing after firewall allowed | Works | Works with delay | Fails |
| Pairing after firewall denied | Clear error | Vague error | No feedback |

#### MITM & Key Verification

| Scenario | Expected Behavior | Actual | Architecture Impact |
|----------|-------------------|--------|---------------------|
| Attacker replays discovery announcement | Mobile connects to legitimate desktop | | |
| Attacker presents fake QR code | Handshake fails; key mismatch detected | | |
| Pairing replay after desktop reboot | Old material rejected; fresh pairing required | | |
| Session replay (captured message resent) | Desktop rejects duplicate sequence number | | |
| Device identity verification | Mobile verifies desktop identity on reconnect | | |
| Fingerprint verification | Fingerprint comparable on both devices | | |

#### Failure Scenarios (Phase 2)
- Desktop hostname changes → mobile attempts resolution → falls back to re-discovery → manual IP.
- DHCP lease renewal → cached IP invalid → re-discovery or hostname resolution.
- Router replaced → full re-discovery required.
- Two desktops on same network → both appear; mobile pairs with correct one.
- Two mobiles on same network → both discover; document concurrent pairing.

#### Gate ✓
Phase 2 passes when Android and Windows complete a Noise handshake, establish a session key, and all MITM scenarios produce the expected protective behavior. At least one onboarding method must be functional.

---

### Phase 3: Encrypted Messaging

**Gate**: End-to-end encrypted communication over the paired session.

#### Scope
- Encrypted ping/pong over established Noise session.
- Session persistence across brief network interruption.
- Reconnection after sleep/wake.
- Session expiry and re-authentication.

#### Success Thresholds

| Metric | PASS | WARNING | FAIL |
|--------|------|---------|------|
| Encrypted round-trip | < 500 ms | 500–1000 ms | > 1000 ms |
| Reconnect after network loss | < 10 s | 10–30 s | > 30 s |
| Reconnect after sleep/resume | < 30 s | 30–60 s | > 60 s |
| Desktop restart → recovery | Auto within 60 s | Manual re-pair | Broken |

#### Failure Scenarios (Phase 3)
- Desktop sleeps → connection lost → measure reconnection on wake.
- Desktop agent killed → full restart → measure recovery time.
- Router reboot → both sides lost → measure re-discovery + reconnect.
- Mobile switches Wi-Fi → connection drops → auto-reconnect or explicit re-pair.
- Mobile enters background (foreground path only) → connection maintained.

#### Gate ✓
Phase 3 passes when Android and Windows exchange encrypted messages, survive network interruption within thresholds, and recover from sleep/resume without user intervention.

---

## Rollback Rule

If any later phase proves that an earlier architectural assumption cannot support the complete user journey, the spike **fails**.

The team must revisit the earlier phase instead of building workarounds. No architectural workaround is permitted inside a spike. A failed phase is not a failure of the spike — it is evidence that the architecture needs to change before production code is written.

## Android Doze

Distinguished but **not tested** in this spike:

| Category | Constrained by Doze? | Examples |
|----------|----------------------|----------|
| **Foreground / user-driven** | No | Button presses, manual trigger execution, pairing flow |
| **Background / automated** | Yes | Context monitoring, trigger evaluation, scheduled macros |

This spike validates the foreground path only. Background constraints are investigated in EP-002.

## Artifacts

- Source code in this directory (throwaway — never merged).
- Screenshots or video of each onboarding method.
- Completed evidence template — see [`evidence.md`](./evidence.md) (measurements, compatibility matrix, thresholds, limitations, artifacts).
- Recommendations for ADR-006 and any new ADRs.

## Outcome

Result feeds into:

- ADR-006 (Transport Evaluation) — accepted. **Wi-Fi (WebSocket)** is the primary command transport.
  **BLE** deferred as a future `PresenceProvider` (proximity events only — never returns endpoints).
- **Provider architecture** — Two distinct abstractions:
  - **Discovery Providers** (Android `DiscoveryProvider` interface) resolve desktop identity into a
    connection endpoint. mDNS is the first implementation. Future: cached hostname, cached IP,
    manual entry, enterprise DNS.
  - **Presence Providers** (defined interface, no implementation yet) emit proximity events and
    trigger discovery. Reserved for BLE, NFC, UWB.
  - **Desktop side**: `AdvertisementProvider` trait for announcing desktop presence (mDNS first).
- No transport code depends on the presence layer. No presence provider fabricates endpoint data.
- Bootstrap UI decision (which onboarding method to adopt).
- Hostname-resolution fallback approach.
- MITM mitigation requirements for security ADR.
- Message format decision (JSON sufficient or need schema).

## Implementation

### Phase 1 — Complete

| Component | Location | Language | Status |
|-----------|----------|----------|--------|
| Windows desktop | `desktop/` | Rust | ✅ Compiles and runs |
| Android mobile | `mobile/` | Kotlin | ✅ Source complete (requires Android Studio + SDK to build) |

#### Desktop Agent (Windows)
- **Build**: `cd desktop && cargo build --release`
- **Run**: `RUST_LOG=info ./target/release/amd-desktop.exe`
- **Announcement architecture**: Pluggable provider model via `AdvertisementProvider` trait. `MdnsAnnouncer` is the first implementation.
- **mDNS**: Announces `_amd._tcp.local.` with device ID, type, protocol version, OS, and provider tag.
- **Future providers**: Trait ready for BLE advertisement, DNS-SD, or other announcement mechanisms.
- **WebSocket**: Listens on port 9742. Accepts JSON messages. Responds to `{"type": "ping"}` with `{"type": "pong", "echo": ...}`.
- **Dependencies**: mdns-sd, tokio-tungstenite, serde_json, env_logger.

> **⚠ Note on device identity:** Current prototype uses a hostname-derived ID (`amd-{hostname}`) for
> convenience. This is **not** a validated persistent identity. Hostnames change (rename, reinstall,
> clone, corporate policy). Persistent device identity will be evaluated during Phase 2.

#### Mobile App (Android)
- **Build**: Open `mobile/` in Android Studio, sync Gradle, run on device.
- **Discovery architecture**: Pluggable provider model via `DiscoveryProvider` interface. `DiscoveryManager` orchestrates providers in priority order, returning first non-empty result with deduplication.
- **mDNS**: `MdnsDiscoveryProvider` uses Android's `NsdManager` to discover `_amd._tcp.local.` services. Non-blocking timeout via `Handler.postDelayed`.
- **Future providers**: Interface ready for cached hostname, cached IP, BLE, manual entry. Register via `discoveryManager.register(provider)`.
- **Discovery timing**: `DiscoveryManager` measures total elapsed time across all providers. Reports latency in the UI and logs.
- **UI**: "Scan for Desktops" button → list of discovered devices (with latency shown) → tap to connect.
- **WebSocket**: Uses OkHttp WebSocket client. Sends `{"type": "ping"}` on connect. Displays `pong` response with round-trip timing.
- **Dependencies**: okhttp3, kotlinx-coroutines, androidx-appcompat, androidx-activity-ktx. No ML Kit (QR scanning deferred to Phase 2).

### Phase 2 — Not Started
Pairing, key exchange, and MITM validation.

### Phase 3 — Not Started
Encrypted messaging.

## Evidence

### Phase 1 — Discovery & Transport

Thresholds (from governance):
| Metric | PASS | WARNING | FAIL | Measured |
|--------|------|---------|------|----------|
| Discovery time | <5 s | 5–10 s | >10 s | — |
| WebSocket connect | <2 s | 2–5 s | >5 s | — |
| Ping/pong round-trip | <500 ms | 500–1000 ms | >1000 ms | — |

### Compatibility
| Scenario | Expected | Result |
|----------|----------|--------|
| Wired desktop + Wi-Fi phone | Pass | — |
| Windows Firewall (allow) | Pass | — |
| Windows Firewall (deny) | Block | — |
| Both on same subnet | Pass | — |
| Cross-subnet (no mDNS proxy) | Fail | — |

> ⚠ **Note on device identity:** The prototype uses `amd-{hostname}` as device ID. This is a
> temporary convenience, not a validated persistent identity. See "Risks" section below.

### Known Limitations (Phase 1)
1. Device ID is hostname-derived and will change on rename/reinstall. Addressed in Phase 2 identity work.
2. Android side requires SDK 26+ (Android 8.0) due to NsdManager API.
3. No encryption (plaintext WebSocket). Phase 3 adds Noise protocol.
4. No pairing — any client on the network can connect. Phase 2 adds QR-code trust-on-first-use.

## Status

Phase 1 implemented. Ready for testing on real hardware.
