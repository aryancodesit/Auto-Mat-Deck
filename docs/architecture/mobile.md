# Mobile Architecture

## Status

The production mobile application has **not yet been developed**.

## Current implementation (v0.4 spike)

Current mobile code lives in `spikes/ep-001-discovery-pairing/mobile/`
for experimentation only — it validates mDNS discovery, WebSocket
connection, and `active_profile_state` projection reception. It is **not**
the production app.

### v0.4 topology

```
Desktop Agent
    │
    │ WebSocket server
    ▼
  network
    ▲
    │ OkHttp WebSocket client
    │
Android discovery/pairing spike
```

Desktop is always the WebSocket server. The Android spike connects to the
Desktop using OkHttp — it is never a WebSocket server.

### Spike file inventory

```
spikes/ep-001-discovery-pairing/mobile/
├── app/
│   └── src/main/java/com/automatdeck/spike/
│       ├── MainActivity.kt              # Entry point, permissions, UI, WebSocket client
│       ├── ActiveProfileStateMessage.kt  # Strict v1 projection parser
│       ├── MdnsDiscoveryProvider.kt     # mDNS discovery of Desktop instances
│       ├── DiscoveryProvider.kt          # Discovery abstraction
│       ├── DiscoveryManager.kt          # Discovery coordination
│       ├── DiscoveredDevice.kt          # Discovered device model
│       └── DeviceAdapter.kt             # Device list UI adapter
├── app/src/test/java/com/automatdeck/spike/
│   └── ActiveProfileStateMessageTest.kt # 18 unit tests
├── build.gradle.kts
└── settings.gradle.kts
```

### Current spike capabilities

- **mDNS discovery** — Discovers Desktop instances on the LAN.
- **WebSocket client** — Connects to Desktop using OkHttp `WebSocketClient`.
- **`active_profile_state` reception** — Parses the v1 projection payload.
- **Strict validation** — Rejects unsupported or invalid `schema_version`.
- **Memory retention** — Retains the latest accepted projection in
  Activity memory.
- **No projection persistence** — State is lost on process death.
- **No context observation or profile resolution** — Desktop is the sole
  authority.

This spike is disposable. It will be discarded or rewritten for the
production app. It is **not** the production Android client.

## Target architecture (EP-005+)

```
app/src/main/java/com/automatdeck/
├── MainActivity.kt      # Entry point, permission requests
├── discovery/           # mDNS discovery of Desktop instances
├── connection/          # WebSocket client to Desktop
├── pairing/             # OTP generation & display
├── actions/             # Action request UI & invocation
├── devices/             # Paired device list (local cache)
└── ui/                  # Jetpack Compose screens
```

## Key decisions (deferred until EP-005)

- **Navigation** — Single-activity with Jetpack Compose destinations.
- **Local storage** — DataStore or Room for paired-device cache.
- **Background discovery** — `NetworkServiceDiscovery` or JmDNS.
- **Permissions** — Nearby devices (N), Internet, foreground service.

## Relationship to Desktop

The Mobile app is **always a client**:

- It initiates connections (after discovering Desktop via mDNS).
- It never stores configuration — Desktop is the authority.
- It displays information and triggers actions; Desktop executes them.

## Build (future)

```powershell
cd apps\mobile
.\gradlew assembleDebug
```
