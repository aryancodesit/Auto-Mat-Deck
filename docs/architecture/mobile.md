# Mobile Architecture

## Status

The production mobile application has **not yet been developed**.

## Current implementation (spike)

Current mobile code lives in `spikes/ep-001-discovery-pairing/mobile/`
for experimentation only — it validates mDNS discovery and WebSocket
transport. It is **not** the production app.

```
spikes/ep-001-discovery-pairing/mobile/
├── app/
│   └── src/main/java/com/automatdeck/spike/
│       ├── MainActivity.kt    # Entry point, permissions, UI
│       ├── MobileAdvertiser   # mDNS advertising (WIP)
│       └── MobileWsServer.kt  # WebSocket server (WIP)
├── build.gradle.kts
└── settings.gradle.kts
```

This is experimental code. It will be discarded or rewritten for the
production app.

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
