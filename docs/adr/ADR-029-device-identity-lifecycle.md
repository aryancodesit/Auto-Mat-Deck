# ADR-029: Device Identity Lifecycle

**Status:** Accepted
**Date:** 2026-07-17
**Supersedes:** —
**Superseded By:** —

## Context

Every connection component (pairing, trust, session, reconnect) needs to
identify the Android device. Without a stable identity, trust cannot
persist across restarts, reconnect cannot resume sessions, and pairing
cannot be verified.

The spike code used an unstable identity derived from
`"Android-${Build.MODEL}"` — this changes with device model and
provides no cryptographic uniqueness.

## Decision

### 1. DeviceIdentity generates a UUID v4 on first launch

```kotlin
class DeviceIdentity(context: Context) {
    val deviceId: String   // UUID v4, persisted permanently
    val deviceName: String // human-readable, user-configurable
}
```

UUID v4 provides 128-bit randomness. Collision probability is
negligible for a single-user application.

### 2. Identity is persisted in SharedPreferences

```
device_identity/
  ├── device_id: String (UUID v4)
  ├── device_name: String ("Android-${Build.MODEL}")
  └── paired: Boolean
```

SharedPreferences is sufficient for a single-device identity. No
database, no file management, no encryption (identity is not a secret
— it's an identifier).

### 3. Identity never regenerates unless explicitly reset

The identity survives:
- App restart
- Phone reboot
- WiFi reconnection
- App update

The identity is only reset when:
- User explicitly unpairs/resets pairing
- User factory-resets the app

Reset generates a new UUID v4 and invalidates all existing trust on
the Desktop side.

### 4. Identity is used throughout the connection subsystem

| Component | Usage |
|-----------|-------|
| PairingManager | `device_id` in `identify` and `pair_request` messages |
| TrustedDeviceStore | Stored as `device_id` in trust records |
| SessionManager | Session restoration key |
| ReconnectManager | Reconnection identity verification |
| DiscoveryCache | Cached alongside last-known address |

### 5. Desktop receives identity in the `identify` message

```json
{
    "type": "identify",
    "device_id": "550e8400-e29b-41d4-a716-446655440000",
    "device_name": "Android-Pixel 7"
}
```

The Desktop uses `device_id` to look up trust in TrustStore. If
trusted, the device is admitted. If not, pairing is required.

## Consequences

### Positive

- Stable identity across all connection lifecycle events
- Trust persists correctly (linked to device_id, not IP/name)
- Reconnect works (device_id is the session restoration key)
- Simple implementation (SharedPreferences, one class)
- Reset behavior is explicit and user-controlled

### Negative

- SharedPreferences is not encrypted (acceptable for an identifier)
- No identity migration from spike code (existing paired devices need
  re-pairing — acceptable since spike was not production)

### Risks

- If SharedPreferences is cleared (by user or OS), identity is lost.
  All trust is invalidated. User must re-pair. This is the expected
  behavior for a cleared app data scenario.

## Compliance

- DeviceIdentity MUST generate UUID v4 on first launch
- DeviceIdentity MUST persist identity permanently
- DeviceIdentity MUST NOT regenerate unless explicitly reset
- All connection components MUST use DeviceIdentity.deviceId
- Desktop MUST use device_id (not IP, not name) for trust verification
- Reset MUST be explicit user action, never automatic
