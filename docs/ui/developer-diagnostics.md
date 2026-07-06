# Developer Diagnostics Mode

**Status:** Design proposal (not yet implemented)
**Target:** Mobile app v0.3+ (Android application skeleton)

> **Current focus:** EP-001 hardware validation. Diagnostics are not being built yet.
> This document captures the long-term roadmap so the architecture is consistent from day one.

## Principle

Diagnostics evolve alongside complexity. Each milestone exposes only the information needed to debug that milestone. Adding fields before the corresponding subsystems exist creates noise, not insight.

## Access

- **Debug builds only.** Gated by `BuildConfig.DEBUG`.
- Entry point: long-press version label in Settings → "Developer Diagnostics" toast → tap to open.
- No icon or menu entry in release builds.

---

## EP-001 (Discovery & Transport)

Minimal view during the first validation spike.

```
┌─────────────────────────────────────┐
│  Developer Diagnostics        [✕]   │
├─────────────────────────────────────┤
│  Connection                        │
│  ┌───────────────────────────────┐  │
│  │ Provider    │ mDNS            │  │
│  │ Endpoint    │ 192.168.1.20   │  │
│  │             │ :9742           │  │
│  │ State       │ ● Connected     │  │
│  │ RTT (last)  │ 12 ms           │  │
│  │ RTT (avg)   │ 14 ms           │  │
│  │ Samples     │ 143            │  │
│  └───────────────────────────────┘  │
│                                     │
│  Last Ping                          │
│  ┌───────────────────────────────┐  │
│  │ 12:00:01 → Ping               │  │
│  │ 12:00:01 ← Pong (4ms)        │  │
│  └───────────────────────────────┘  │
│                                     │
│  Log                                │
│  ┌───────────────────────────────┐  │
│  │ [12:00:01] WebSocket opened   │  │
│  │ [12:00:01] Ping sent          │  │
│  │ [12:00:01] Pong received      │  │
│  │ [12:00:05] Keepalive tick     │  │
│  └───────────────────────────────┘  │
│                                     │
│  [  Export Log  ]                   │
└─────────────────────────────────────┘
```

| Field | Source |
|-------|--------|
| Provider | `DiscoveryManager.currentProvider.name` |
| Endpoint | `TransportManager.remoteAddress` |
| State | `TransportManager.connectionState` |
| RTT | Rolling window (last 200 pings) |
| Last Ping | Last sent/received message with duration |
| Log | In-memory ring buffer (last 200 entries) — export via share intent |

---

## v0.4 (Communication Layer — adds Pairing)

Add pairing section.

```
│  Pairing                            │
│  ┌───────────────────────────────┐  │
│  │ State       │ Paired          │  │
│  │ Device ID   │ amd-aryan-pc   │  │
│  │ Paired at   │ 2026-07-06     │  │
│  └───────────────────────────────┘  │
```

---

## v0.5+ (Profiles, Actions, Macros)

Add performance counters and command history.

```
│  Performance                        │
│  ┌───────────────────────────────┐  │
│  │ RTT (max)   │ 87 ms           │  │
│  │ RTT (min)   │ 3 ms            │  │
│  │ TX          │ 1,204 bytes    │  │
│  │ RX          │ 892 bytes      │  │
│  │ Reconnects  │ 2               │  │
│  │ Uptime      │ 00:18:42       │  │
│  └───────────────────────────────┘  │
│                                     │
│  Last Command                       │
│  ┌───────────────────────────────┐  │
│  │ 12:00:01 → ExecuteAction      │  │
│  │ 12:00:01 ← Result OK (13ms)   │  │
│  │ 12:00:05 → Ping               │  │
│  │ 12:00:05 ← Pong (4ms)        │  │
│  └───────────────────────────────┘  │
```

---

## v0.6+ (Encryption — adds pairing details)

```
│  Fingerprint │ A1:B2:C3:...   │  │
```

---

## v0.6 (Encryption — adds clock correlation)

- **Clock delta** — estimate phone↔desktop clock offset via handshake timestamps (approximate, not NTP-accurate). Introduced here because encryption adds handshake timing, retries, key exchange, and session expiration — clock alignment helps correlate logs across devices during debug.

## v1.0+ (Advanced Diagnostics)

Features deferred until post-launch stability is established:

- **Export Debug Bundle** — single button creates `debug_bundle.zip` containing:
  - `desktop.log`
  - `mobile.log`
  - `connection.json`
  - `pairing.json`
  - `diagnostics.json`
  - `metadata.json`
- **Plugin timings** — per-plugin execution duration histogram.
- **Automation traces** — trigger → condition → action waterfall.

---

## Sensitive Logging

All diagnostics output must be sanitized. Do not rely on developer discipline — encode it in the type system:

```kotlin
// Instead of logging raw values everywhere, add a redacted representation:
data class TypeText(val text: String) {
    fun toLogString() = "TypeText(length=${text.length})"
}
```

All action/event types should implement a `toLogString()` (or `redacted()`) method that produces a safe representation. This makes sanitization automatic — no developer has to remember to do it.

| Do | Don't |

| Do | Don't |
|----|-------|
| `TypeText(length=12)` | `TypeText("password")` |
| `LaunchApp("com.example.app")` | `LaunchApp("SecretProject")` |
| `KeyboardAction(hidden=true)` | `KeyboardAction(input="secret")` |
| Log file paths from `context.filesDir` | Log full user home paths (`C:\Users\Aryan\...`) |
| Log error codes | Log message payloads |

Clear-text secrets (passwords, tokens, key material) must never appear in diagnostics, logs, or export bundles.

---

## Build Gating

```kotlin
if (BuildConfig.DEBUG) {
    // Register diagnostics activity
    // Add long-press listener to version label
} else {
    // No diagnostics code in classpath
}
```

Diagnostics activity not registered in the manifest for release builds. `BuildConfig.DEBUG` is `false` in release, so entry point code is dead-stripped by R8/ProGuard.

---

## Rationale

- Dedicated debug overlay avoids ad-hoc logging during hardware testing.
- Incremental rollout prevents information overload — each milestone adds only relevant fields.
- Ring buffers (200 entries) prevent unbounded memory growth.
- Sanitized logging prevents accidental secret exposure.
- Export bundle + screen recording enables full asynchronous debugging.
