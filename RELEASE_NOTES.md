# RELEASE_NOTES

## EP-001 — Discovery & Transport Spike

> **Tag:** `v0.1-ep001-certified`
> **Date:** 2026-07-06
> **Status:** PASS — Environment, discovery, connection, and transport validated on real hardware. Stability, stress, failure, and repeatability intentionally deferred to EP-002.

### Objective

Validate that a Windows desktop agent and an Android mobile app can discover each other over local Wi-Fi, establish a WebSocket connection, and exchange JSON messages — on real hardware, with no cloud dependency.

### Hardware

- **Desktop:** Aryan Gupta's Windows PC (192.168.29.59)
- **Mobile:** realme RMX3392, Android 14 (API 34) (192.168.29.56)
- **Network:** Same Wi-Fi subnet (5 GHz band)

### Discovery Validation

| Metric | Result | Threshold |
|--------|--------|-----------|
| mDNS discovery (desktop → phone) | ~67 ms resolved | PASS (<5 s) |
| Service type: `_amd._tcp.` | Fixed `NsdManager` format | APP-001 resolved |

Provider architecture validated: `DiscoveryProvider` interface → `DiscoveryManager` orchestrator → `MdnsDiscoveryProvider` (Android). Desktop uses `AdvertisementProvider` trait + `MdnsAnnouncer`.

### WebSocket Transport Validation

| Phase | Result |
|-------|--------|
| WebSocket connection (`ws://192.168.29.59:9742/`) | PASS |
| JSON ping/pong round-trip | PASS (RTT <500 ms) |
| Desktop agent (Rust, port 9742) | Compiles, runs, no panics |

### Major Bug Fixes

- **APP-001:** `NsdManager` service type `"_amd._tcp.local."` → `"_amd._tcp."` (NsdManager appends `.local.` internally). Fixed and verified.
- **ENV-001:** Missing `androidx.recyclerview:recyclerview:1.3.2` added to `build.gradle.kts`. Resolved 9 cascade compile errors.
- **DESK-001:** `Cargo.lock` not included in desktop `.gitignore` (won't fix — spike policy).
- `DeviceAdapter.kt`: `class ViewHolder` → `inner class ViewHolder`
- `MainActivity.kt`: Operator precedence with elvis + `System.currentTimeMillis()`

### Known Limitations

- Discovery timeout fixed at 5 seconds via `Handler.postDelayed` (no configurable TTL).
- UI shows connection status only in `statusText` and `responseText` labels — no dedicated connection state widget.
- No desktop-side connection logging (Android-side logging exists).
- Logging at INFO level in production-prone `MdnsDiscoveryProvider` (needs demotion).
- Only single device discovery tested; multi-device and device disappearance untested.
- APK is debug-unsigned; no release signing configured.
- `apps/desktop` and `apps/mobile` directories are empty placeholders — all spike code lives in `spikes/`.
- CI workflows are placeholders (no actual build/lint steps).

---

## EP-002 — Pairing & Trust Spike

> **Tag:** `v0.1-ep002-certified`
> **Date:** 2026-07-06
> **Status:** PASS — Pairing, trust store, auto-reconnect, and unknown device rejection validated on real hardware.

### Objective

Add a trust layer between discovery and execution. A phone must pair with the desktop once, then reconnect automatically without re-pairing. Unknown devices are rejected.

### Protocol Messages

| Direction | Type | Purpose |
|-----------|------|---------|
| Phone → Desktop | `identify` | Announce device_id + device_name |
| Desktop → Phone | `trusted` / `untrusted` | Admission gating |
| Phone → Desktop | `pair_request` | Initiate pairing |
| Desktop → Phone | `pair_accepted` / `pair_rejected` | Pairing result |

### Hardware Validation

| Test | Result |
|------|--------|
| Pair request → console approval → paired | ✅ PASS (11 ms RTT) |
| Trusted device auto-reconnect after app + agent restart | ✅ PASS |
| Trust reset (delete `trusted_devices.json`) → re-pair | ✅ PASS |

### Key Components

- **Desktop (Rust):** Trusted device store at `%APPDATA%/AutoMatDeck/trusted_devices.json`
- **Mobile (Android):** UUID identity persisted in SharedPreferences, Pair button shown on `untrusted`

---

## EP-002.5 — Desktop Packaging

> **Status:** ✅ Done (embedded in EP-001 code)

- System tray icon (tray-icon 0.19.3) with menu: Status, Open Logs, Exit
- File logging to `%APPDATA%/AutoMatDeck/agent.log`
- Windows subsystem (`#![windows_subsystem = "windows"]`) in release builds
- CLI: `--install` / `--uninstall` for Windows startup registry key
- Single-instance enforcement via named mutex
- Start with Windows checkable tray menu item (registry toggle)
- Tray-based pairing approval with dynamic menu rebuild (no stdin dependency)
- Release build verified: 3.3 MB, tray icon visible, logging works

---

## EP-003 — Remote Actions

> **Tag:** `v0.1-ep003-certified`
> **Date:** 2026-07-07
> **Status:** PASS — All 5 actions executed successfully from a trusted device.

### Objective

Prove a trusted phone can ask the desktop to perform exactly 5 actions: launch app, open URL, open file, lock workstation, show notification.

### Protocol

Request:
```json
{"type":"action","request_id":"...","action":"launch","payload":{"app":"chrome"}}
```

Response:
```json
{"type":"action_result","request_id":"...","success":true,"data":{...}}
```

### Architecture

```
desktop/actions.rs  ← Action trait, ActionRegistry, 5 implementations
```

`ActionRegistry` with `HashMap<&str, Box<dyn Action>>` — each action implements `execute(&self, &Value) -> Result<Value, ActionError>`.

### Hardware Validation

| Action | Result |
|--------|--------|
| `launch chrome` | ✅ Chrome opened (PID returned) |
| `open_url github.com` | ✅ Browser opened |
| `open_file calc.exe` | ✅ Calculator launched |
| `lock` | ✅ WorkStation locked |
| `notify` | ✅ Windows toast notification appeared |

---

## v0.1 Release

With EP-001 (Discovery), EP-002 (Trust), EP-002.5 (Packaging), and EP-003 (Actions) certified, v0.1 is complete.

### Stack

```
Android App
     │
     ▼
Discovery (mDNS)
     │
     ▼
WebSocket
     │
     ▼
Trust Layer (trusted_devices.json)
     │
     ▼
Desktop Agent (system tray, file logging)
     │
     ▼
Remote Action Engine (5 actions)
```

### What was proven

- Discovery, transport, and trust work on real hardware with no cloud dependency
- Desktop agent runs as a proper Windows application (tray icon, file logging, no console, single instance)
- Pairing is user-friendly (tray-based approval, no console required)
- Five remote actions execute reliably from a trusted phone

### Not in v0.1

- BLE presence (deferred to future PresenceProvider)
- ConnectionManager (needed when BLE arrives)
- Unpair management and pair list UI (v0.2)
- Additional actions beyond 5 (v0.2+)
- Macros, plugins, scripting (v0.4+)
- Multi-device and multi-user support

---

## v0.5 Release

> **Tag:** `v0.5.0`
> **Branch:** `v0.5` (merged from `feature/v0.5-control-surface`)
> **Date:** 2026-07-16

### Objective

Replace the legacy `action` message type with a Desktop-authoritative control surface. The Desktop owns the profile model and projects an opaque control surface to connected Mobile clients. Mobile requests invocation by opaque button_id; the Desktop validates and executes.

### Architecture

```
Mobile                          Desktop
  │                               │
  │◄── control_surface_state ─────│  projection (ADR-021)
  │    (pages, buttons, opaque)   │
  │                               │
  │── control_invoke ────────────►│  validation (Sprint 3)
  │   {button_id}                 │  execute (Sprint 4)
  │                               │
  │◄── control_invoke_result ─────│  accepted/rejected + executed/failed
  │                               │
```

### Sprint 3 — Validation Transport

- `validate_button()`: pure function resolving button_id against active profile
- Rejection reasons: `unknown_button`, `no_active_profile`, `ambiguous_button`
- `control_invoke` handler in agent.rs: transport-only, no execution
- Android: `ControlInvokeRequest` outbound, `SpikeMessageDispatcher` inbound parsing
- ADR-022: Opaque Control Invocation protocol decision record

### Sprint 4 — Execution Layer

- `ExecutionOutcome` enum: Success, Failed, ActionNotFound, Timeout, Panicked
- `execution.rs`: async wrapper — `spawn_blocking` + `tokio::time::timeout(5s)` + `catch_unwind`
- Extended response schema: `executed` (bool), `execution_error` (string)
- Android: dispatcher parses `executed`/`execution_error` fields
- ADR-022 updated with execution semantics, failure taxonomy, compliance rules

### Protocol Additions

| Field | Type | When | Added |
|-------|------|------|-------|
| `accepted` | bool | always | Sprint 3 |
| `reason` | string? | accepted=false | Sprint 3 |
| `executed` | bool? | accepted=true | Sprint 4 |
| `execution_error` | string? | accepted=true, executed=false | Sprint 4 |

Execution error codes: `execution_failed`, `action_not_found`, `execution_timeout`, `execution_panicked`

### ADR Updates

- **ADR-021:** Control Surface Projection — Desktop-authoritative projection model
- **ADR-022:** Opaque Control Invocation — validation boundary, execution semantics, failure taxonomy

### Test Coverage

| Suite | Count | Status |
|-------|-------|--------|
| Desktop (Rust) | 225 | ✅ passing |
| Android (Kotlin) | All | ✅ passing |

Desktop tests span: agent, command, editor, execution, model, observer, pairing, projection, projection_transport, state.

Android tests span: ActiveProfileStateMessage, ControlInvokeRequest, ControlSurfacePresentationMapper, ControlSurfaceStateMessage, SpikeMessageDispatcher (Path A-G).

### Files Changed (v0.5)

**Desktop (Rust):**
- `src/actions.rs` — ExecutionOutcome enum
- `src/agent.rs` — control_invoke handler with execution dispatch
- `src/execution.rs` — async execution wrapper (new)
- `src/main.rs` — module registration
- `src/projection.rs` — validate_button(), CSS projection, dedup policy
- `src/projection_transport.rs` — CSS wire format, serialization
- `src/state.rs` — runtime state reconciliation

**Android (Kotlin):**
- `ControlInvokeRequest.kt` — outbound request serialization
- `ControlSurfacePresentationMapper.kt` — projection to native UI
- `ControlSurfaceStateMessage.kt` — inbound CSS parsing
- `SpikeMessageDispatcher.kt` — message routing, result parsing
- `MainActivity.kt` — diagnostic display

**Documentation:**
- `docs/adr/ADR-021-control-surface-projection.md`
- `docs/adr/ADR-022-control-invoke.md`
- `docs/architecture/v0.5-*.md` — scope, protocol, sequences, failure semantics, test matrix
- `docs/sprint-4-execution-design.md`

### Known Limitations

- 5s hard timeout on action execution (acceptable for HID/GPIO; revisit for long-running actions)
- No retry logic (fire-and-forget for OS actions)
- No rollback (OS actions are irreversible)
- Execution is single-shot (no action queue or scheduling)
- `ActionRegistry.execute()` is synchronous (wrapped in spawn_blocking)

### Not in v0.5

- Workflow engine
- Macro chaining
- Triggers
- Scheduler
- Context automation
- Multi-device execution
- Plugin system
- Undo/rollback
- Retry logic
- Execution history
- Action queue
- Analytics
