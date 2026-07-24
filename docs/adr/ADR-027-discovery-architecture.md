# ADR-027: Discovery Architecture

**Status:** Accepted
**Date:** 2026-07-17
**Supersedes:** —
**Superseded By:** —

## Context

v0.7 relied on spike discovery code in `spikes/ep-001-discovery-pairing/`.
This code was validated on real hardware but is not production-ready: it's
single-shot only, has no caching, and lives in a disposable spike directory.

v0.8 needs production-quality discovery that supports:
- Continuous/background scanning
- Last-known device caching for fast reconnection
- Clean separation from pairing and trust

## Decision

### 1. Discovery module lives in apps/mobile/discovery/

```
apps/mobile/app/src/main/java/com/automatdeck/app/discovery/
  ├── DiscoveredDevice.kt     — data class with address, name, deviceId
  ├── DiscoveryProvider.kt    — interface for discovery backends
  ├── MdnsDiscoveryProvider.kt — mDNS implementation (NsdManager)
  ├── DiscoveryCache.kt       — SharedPreferences-based last-known cache
  └── DiscoveryManager.kt     — orchestrator with continuous scan support
```

### 2. DiscoveryProvider is an interface

```kotlin
interface DiscoveryProvider {
    suspend fun discover(timeoutMs: Long = 5000L): List<DiscoveredDevice>
    fun stop() {}
}
```

Enables future providers (UDP broadcast, BLE) without changing the manager.
The interface is `suspend` for coroutine-native integration.

### 3. DiscoveryCache stores last-known device

SharedPreferences-based. Stores host, port, name, deviceId, and
discoveredAt timestamp. Used for fast reconnection on app launch —
skip discovery if last-known device is reachable.

### 4. DiscoveryManager supports continuous scanning

```kotlin
fun startContinuousScan(scope: CoroutineScope, intervalMs: Long = 10_000L)
fun stopContinuousScan()
```

Periodic scan with configurable interval. Callbacks for device found
and discovery complete. Caches first device found.

### 5. mDNS service type remains `_amd._tcp.`

Proven on real hardware (EP-001). NsdManager on Android, mdns-sd on
Desktop. No changes to the wire format.

## Consequences

### Positive

- Production-quality discovery (was spike code)
- Continuous scanning for background discovery
- Last-known cache enables fast reconnection
- Clean interface for future discovery backends
- Separated from pairing and trust concerns

### Negative

- SharedPreferences-based cache is not encrypted (acceptable for device address data)
- Continuous scanning uses battery (configurable interval mitigates)

### Risks

- mDNS may not work on restrictive networks. QR pairing is the fallback.
- NsdManager behavior varies across Android versions. Tested on API 34.

## Compliance

- Discovery MUST use `_amd._tcp.` service type
- DiscoveryCache MUST store device address for reconnection
- Continuous scan MUST be cancellable via stopContinuousScan()
- DiscoveryProvider MUST be interface-based for extensibility
