# EP-002.5 — Desktop Packaging Evidence

> **Status:** ✅ Verified  
> **Date:** 2026-07-07  
> **Platform:** Windows 11 x64, Rust 1.91.1

---

## Test Results

| # | Test | Expected | Result |
|---|------|----------|--------|
| 1 | Release binary builds | `cargo build --release` succeeds with `#![windows_subsystem = "windows"]` | ✅ PASS |
| 2 | Double-click .exe | Tray icon appears in notification area, no console window | ✅ PASS |
| 3 | File logging | `%APPDATA%/AutoMatDeck/agent.log` created with structured log entries | ✅ PASS |
| 4 | CLI: `--install` | Registry key set at `HKCU\...\Run\AutoMatDeck Agent` with exe path | ✅ PASS |
| 5 | CLI: `--uninstall` | Registry key removed, "Auto-start removed." printed | ✅ PASS |
| 6 | Single-instance | Second instance prints "AutoMatDeck Agent is already running." and exits | ✅ PASS |
| 7 | Tray menu: Exit | Clicking Exit in tray menu terminates agent gracefully | ✅ PASS |
| 8 | Tray menu: Open Logs | Opens Explorer to `%APPDATA%/AutoMatDeck/` | ✅ PASS |
| 9 | WebSocket server | Server still listens on port 9742, accepts trusted connections | ✅ PASS |
| 10 | Trusted device reconnect | Previously paired phone auto-connects without re-pairing | ✅ PASS |
| 11 | Debug build | Console visible + file logging + tray icon (dev mode) | ✅ PASS |

---

## Log Output (Sample)

```
[2026-07-07T07:07:36Z INFO  amd_desktop] AutoMatDeck Agent starting...
[2026-07-07T07:07:36Z INFO  amd_desktop] Log file: C:\Users\aryan\AppData\Roaming\AutoMatDeck\agent.log
[2026-07-07T07:07:36Z INFO  amd_desktop] System tray icon active.
[2026-07-07T07:07:36Z INFO  amd_desktop::discovery] [mDNS] Advertising amd-AryanGupta as AutoMatDeckDesktop._amd._tcp.local. on port 9742
[2026-07-07T07:07:36Z INFO  amd_desktop] Desktop agent started. Hostname: AryanGupta, Device ID: amd-AryanGupta, Listening on port 9742
[2026-07-07T07:07:36Z INFO  amd_desktop] WebSocket server listening on ws://0.0.0.0:9742
[2026-07-07T07:39:47Z INFO  amd_desktop] Trusted device connected: Android-RMX3392 (a5d575ed-46f6-40ee-878f-e40057642d27)
```

## Registry Verification

```
Path: HKCU:\Software\Microsoft\Windows\CurrentVersion\Run
Name: AutoMatDeck Agent
Value: D:\Codes\AUTO-MAT-DECK\spikes\ep-001-discovery-pairing\desktop\target\release\amd-desktop.exe
```

## Binary Info

- Path: `target/release/amd-desktop.exe`
- Size: 3,359,232 bytes (3.3 MB)
- Type: Windows executable with `#![windows_subsystem = "windows"]` (no console in release)
- Dependencies: tray-icon 0.19.3, muda 0.15.3 (menu), tokio, mdns-sd, serde, etc.

---

## Changes Made

### `Cargo.toml`
- Added `tray-icon = "0.19"` (system tray icon)
- Added `windows-sys` features: `Win32_System_Threading`, `Win32_System_Registry`, `Win32_Security`

### `src/main.rs`
- Added `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`
- System tray icon with menu: Status (disabled), Open Logs, Exit
- File logging via `env_logger::Target::Pipe` to `%APPDATA%/AutoMatDeck/agent.log`
- CLI args: `--install` (set auto-start registry key), `--uninstall` (remove)
- Single-instance via named mutex `Local\AutoMatDeck_Agent`
- Windows message pump (`PeekMessageW` loop) on main thread
- Tokio runtime moved to separate thread
- Graceful shutdown via `tokio::sync::watch`

### Unchanged
- All EP-002 pairing/trust logic
- mDNS discovery via `MdnsAnnouncer`
- WebSocket protocol and handlers
- Trusted device store (`trusted_devices.json`)
- Console approval prompt (async stdin)
