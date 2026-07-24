# ADR-028: Pairing Architecture

**Status:** Accepted
**Date:** 2026-07-17
**Supersedes:** —
**Superseded By:** —

## Context

Pairing logic was embedded in `MainActivity.kt` (spike code). This made
the pairing flow untestable, coupled to UI, and impossible to reuse in
other Android components (e.g., a ConnectionWizard or Service).

v0.8 needs a standalone PairingManager that handles the pairing protocol
independently of UI.

## Decision

### 1. PairingManager is a state machine

```kotlin
enum class PairingState {
    Idle, WaitingForCode, SendingRequest,
    WaitingForApproval, Paired, Failed, TimedOut
}
```

State transitions are explicit and validated. UI observes state changes
via `onStateChanged` callback.

### 2. TrustedDeviceStore replaces SharedPreferences

```kotlin
class TrustedDeviceStore(context: Context) {
    fun save(device: DiscoveredDevice)
    fun get(): DiscoveredDevice?
    fun isTrusted(deviceId: String): Boolean
    fun clear()
    fun hasTrustedDevice(): Boolean
}
```

Fixes the spike bug: now stores `device_id` (not just host/port).
Stores `paired_at` timestamp for audit.

### 3. Pairing flow is callback-based

```kotlin
class PairingManager(
    onStateChanged: (PairingState) -> Unit,
    onMessage: (String) -> Unit
)
```

- `onStateChanged`: UI observes state for rendering
- `onMessage`: human-readable messages for display
- `sendFn: (String) -> Boolean`: WebSocket send function injected by caller

No direct WebSocket dependency. Can be used with any transport.

### 4. QR scanning is deferred

QR scanning requires CameraX + MLKit integration. The PairingManager
supports OTP-based pairing now. QR support will be added when the full
Android app is built (v0.9+).

### 5. Pairing timeout is 130 seconds

Matches the Desktop's OTP code lifetime (5 minutes). The timeout
cancels the pairing attempt and returns to Idle state.

## Consequences

### Positive

- Pairing logic is testable without UI
- State machine prevents invalid transitions
- TrustedDeviceStore fixes the device_id bug
- Callback-based design enables reuse in Service/Wizard
- Timeout handling prevents stuck states

### Negative

- QR scanning not yet implemented (OTP only)
- PairingManager does not own WebSocket (caller must inject sendFn)

### Risks

- Pairing timeout may be too short on slow networks. Can be adjusted.

## Compliance

- PairingManager MUST validate state transitions
- TrustedDeviceStore MUST store device_id for trust verification
- PairingManager MUST handle timeout gracefully
- PairingManager MUST NOT depend on Android UI classes
