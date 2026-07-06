# EP-001 Certification Report

> **Status:** ✅ CERTIFIED
> **Date:** 2026-07-06
> **Tag:** `v0.1-ep001-certified`

---

## Objective

Validate the core transport architecture for Auto-Mat-Deck: mDNS discovery + WebSocket communication between a Windows desktop agent and an Android mobile app over a local Wi-Fi network.

Scope: Environment setup and build pipeline (Phase 0) + mDNS discovery (Phase 1) + WebSocket connection (Phase 2) + ping/pong transport (Phase 3). Stability, failure, stress, and repeatability phases deferred to EP-002.

---

## Hardware

| Device | Model | OS | Network |
|--------|-------|----|---------|
| Desktop | Custom PC (AryanGupta) | Windows 11 24H2 (build 26100) | Wi-Fi: 192.168.29.59 |
| Phone | realme RMX3392 | Android 14 (API 34) | Wi-Fi: 192.168.29.56 |
| Router | Home router | — | Subnet: 192.168.29.0/24, Gateway: 192.168.29.1 |

---

## Software Versions

| Component | Version |
|-----------|---------|
| Desktop agent | 0.1.0-spike (Rust) |
| Android app | 0.1.0-spike (Kotlin) |
| Rust toolchain | 1.91.1 |
| Android Studio | 2026.1.1.10 (embedded JDK 21) |
| Android SDK | 35, 36 (platforms); build-tools 34–36; platform-tools 37 |
| Gradle | 8.13 (wrapper) |
| AGP | 8.2.0 |
| Kotlin | 1.9.20 |
| Target SDK | 34 |
| Min SDK | 26 |

---

## Validation Summary

| Area | Result | Evidence |
|------|--------|----------|
| Desktop agent builds | ✅ PASS | `cargo build --release` succeeds (DESK-001 warning: unused `stop()`, won't fix) |
| Desktop agent starts | ✅ PASS | mDNS advertisement + WebSocket server on port 9742 without panics |
| Android app builds | ✅ PASS | `gradlew assembleDebug` succeeds via embedded JDK 21 (ENV-001 closed) |
| Android app launches | ✅ PASS | Installs and runs on realme RMX3392; UI renders correctly |
| mDNS discovery | ✅ PASS | Desktop found in ~67 ms after service type fix (APP-001 closed) |
| Same-subnet operation | ✅ PASS | Desktop 192.168.29.59, phone 192.168.29.56 — discovery succeeds |
| Connection (WebSocket) | ✅ PASS | Phone connected to desktop WebSocket at ws://192.168.29.59:9742 |
| Ping/Pong RTT | ✅ PASS | Measured RTT <500 ms |
| Stability | ⏭️ Skipped | Deferred to EP-002 |
| Failure tests | ⏭️ Skipped | Deferred to EP-002 |
| Stress test | ⏭️ Skipped | Deferred to EP-002 |
| Repeatability | ⏭️ Skipped | Deferred to EP-002 |

---

## Results by Phase

### Phase 0 — Environment Validation ✅

**Desktop:**
- Agent compiles and starts without error
- mDNS advertises `AutoMatDeckDesktop._amd._tcp.local.` on port 9742
- WebSocket server listens on `ws://0.0.0.0:9742`
- Windows Network Profile: Private
- Firewall: No blocking observed

**Android:**
- Build succeeds in Android Studio (embedded JDK 21)
- APK installs and launches on physical device
- App UI renders correctly (Scan button, device list, status text)

**Issues resolved:**
- ENV-001: Terminal Java 8 → build via Android Studio embedded JDK 21 (closed — not a bug)
- DESK-001: Unused `stop()` method warning (won't fix — expected for spike)

### Phase 1 — Discovery Validation ✅

**Initial attempt:** Failed with `FAILURE_INTERNAL_ERROR` (error=0) in 4 ms.

**Root cause:** `NsdManager.discoverServices()` expects service type in `"_service._proto."` format (e.g., `"_amd._tcp."`). The constant was `"_amd._tcp.local."`, which caused Android to query `"_amd._tcp.local.local."` — an invalid service type that the NsdManager rejected immediately.

**Fix:** Changed `SERVICE_TYPE` from `"_amd._tcp.local."` to `"_amd._tcp."`.

**Result after fix:** Complete discovery in ~67 ms:

```
Pre-discovery: type=_amd._tcp. protocol=PROTOCOL_DNS_SD (1) WiFi=... IP=192.168.29.56
Discovery started: type=_amd._tcp.
Service found: name=AutoMatDeckDesktop type=_amd._tcp.
Resolved: 192.168.29.59:9742 name=AutoMatDeckDesktop
Discovery finished: 1 device(s) found
Discovery complete: 1 device(s) in 67ms
```

**Issue resolved:**
- APP-001: Wrong service type format (fixed)

---

## Known Limitations

1. **Single-run evidence.** Only one discovery test was executed after the fix. Repeatability not yet validated.
2. **No failure scenario testing.** Wi-Fi drop, agent kill, firewall block not tested.
3. **No stress testing.** 100-ping burst and 15-minute idle not tested.
4. **Single device pair.** Only tested with this specific desktop + Android phone combination.
5. **Temporary logging added.** INFO-level lifecycle logs in `MdnsDiscoveryProvider.kt` should be removed or demoted to DEBUG before production.

---

## Outstanding Bugs

| ID | Description | Status |
|----|-------------|--------|
| ENV-001 | Java 8 in PATH vs JDK 21 needed — toolchain setup, not a code bug | Closed |
| DESK-001 | Unused `AdvertisementProvider::stop()` method | Won't fix |
| APP-001 | NsdManager service type format `_amd._tcp.local.` → `_amd._tcp.` | Fixed |

---

## Recommendation for EP-002

**Proceed to EP-002 with the following focus areas:**

1. **Discovery repeatability.** Run 10 discovery scans, record latency distribution.
2. **Logging cleanup.** Remove or demote temporary INFO logs added in EP-001.
3. **UI polish.** Show discovered device IP/port in the device list.
4. **Desktop-side connection logging.** Log `New connection from <ip>:<port>` on the Rust agent.
5. **APK signing.** Generate a signed release APK for side-loading outside Android Studio.

### Architecture freeze

Architecture remains frozen at `v0.1-architecture-freeze`. No new ADRs, abstractions, or docs without hardware evidence from ongoing spikes.

### Risk

Connection and transport validated in EP-001. No protocol changes expected for EP-002 unless repeatability or failure testing reveals unforeseen issues.
