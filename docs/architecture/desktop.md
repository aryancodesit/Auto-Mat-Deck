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

The target splits agent responsibilities into focused modules: WebSocket
handling (`ws.rs`), pairing (`pairing.rs`), and configuration (`config.rs`)
extracted from their current inline locations.

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
- Requests user approval (via tray notification or GUI) for unknown devices.
- Stores trusted devices for future automatic acceptance.

For the wire protocol (message types, direction, payload), see:
[Wire Protocol](protocol.md)

## Build

```powershell
cd apps\desktop
cargo build --release
```
