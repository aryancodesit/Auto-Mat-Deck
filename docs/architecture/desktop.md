# Desktop Architecture

## Overview

AutoMatDeck Desktop is a single native Windows executable written in Rust.
It acts as the system's authority — it owns configuration, stores device
pairings, executes actions, and verifies OTP codes.

## Threading model

Three concurrent execution contexts:

| Thread / Runtime | Role |
|------------------|------|
| **Main thread** (eframe) | GUI, tray icon, user interaction |
| **Tray pump** | Windows message loop for system-tray events |
| **Tokio runtime** | WebSocket server, mDNS advertisement, OTP verification |

The threads communicate via `tokio::sync` primitives (channels, `Arc<Mutex<>>`).

## Module map

### Current implementation

```
src/
├── main.rs           # Entry point, three-thread bootstrap
├── agent.rs          # WebSocket server, pairing, connection handling
├── gui.rs            # eframe UI panels
├── tray.rs           # System-tray icon, menu, single instance
├── actions.rs        # Action definitions & registry
├── discovery.rs      # mDNS advertisement (libmdns)
└── device_store.rs   # Persistent config & paired device storage
```

Note: WebSocket handling and pairing logic live in `agent.rs`; config storage
is in `device_store.rs`. No standalone `ws.rs`, `config.rs`, or `pairing.rs`
module exists today.

### Target architecture (EP-005+)

```
src/
├── main.rs           # Entry point, three-thread bootstrap
├── agent.rs          # Action execution (native APIs)
├── gui.rs            # eframe UI panels
├── tray.rs           # System-tray icon & menu
├── actions.rs        # Action definitions & router
├── discovery.rs      # mDNS advertisement (libmdns)
├── config.rs         # Persistent config (JSON on disk)
├── ws.rs             # WebSocket per-client handler
├── pairing.rs        # OTP generation & verification
└── device_store.rs   # Persisted paired device registry
```

### Current implementation (v0.2)

```
src/
├── main.rs           # Entry point, three-thread bootstrap
├── agent.rs          # WebSocket server, pairing, connection handling
├── gui.rs            # eframe UI panels (includes pairing tab)
├── tray.rs           # System-tray icon, menu, single instance
├── actions.rs        # Action definitions & registry
├── discovery.rs      # mDNS advertisement (libmdns)
├── pairing.rs        # PairingManager, OTP generation, validation, tests
├── editor.rs         # Action editor with typed forms
├── command.rs        # Command pattern for undoable edits
├── model.rs          # Domain types
├── repository.rs     # Data access layer
├── state.rs          # App state
└── device_store.rs   # Persistent config & paired device storage
```

Pairing (`pairing.rs`) and editor (`editor.rs`, `command.rs`, `model.rs`,
`repository.rs`, `state.rs`) have been added ahead of the target timeline.

## Agent — Action execution

The agent (`agent.rs`) executes remote actions using **native Windows APIs
only** — no shell interpreters:

| Action | API |
|--------|-----|
| Launch / open | `ShellExecuteW` |
| Lock workstation | `LockWorkStation` |
| Toast notification | `winrt-notification` (WinRT) |
| Custom key sequence | `SendInput` |
| Execute command | `CreateProcessW` (direct, not via cmd.exe) |

## Configuration

Config is stored as JSON in the user's app-data directory. Desktop is the
**sole authority** — Mobile never holds or syncs configuration.

## Pairing

Desktop manages the pairing lifecycle:

- Receives incoming connections from mobile devices.
- Evaluates trust by checking against the stored device registry.
- Generates OTP codes for manual entry or QR scanning (v0.2+).
- Validates OTP codes from `pair_request` messages via `PairingManager`.
- Falls back to tray approval when no `pairing_code` is provided.
- Stores trusted devices for future automatic acceptance.

Pairing sessions are one-time use, expire after 5 minutes, and can be cancelled
explicitly from the GUI.

For the wire protocol (message types, direction, payload), see:
[Wire Protocol](protocol.md)

## Build

```powershell
cd apps\desktop
cargo build --release
```
