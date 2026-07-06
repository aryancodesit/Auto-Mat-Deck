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

### Deferred to EP-002

- Discovery repeatability (10 runs with latency distribution)
- Logging cleanup (demote INFO in `MdnsDiscoveryProvider`)
- UI polish (connection state, status indicators)
- Desktop-side connection logging
- Release APK signing
- Phases 4–7 of the 7-phase test plan
