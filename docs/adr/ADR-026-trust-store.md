# ADR-026: TrustStore

**Status:** Accepted
**Date:** 2026-07-17
**Supersedes:** —
**Superseded By:** —

## Context

Trust management was entangled with the Document model (`Document.devices`)
and `AppState` methods. This works but couples trust logic to the
persistence layer and makes it impossible to use trust data independently
(e.g., in a ConnectionManager or session validator).

v0.8 needs a standalone TrustStore that can be shared across connection
components without going through AppState.

## Decision

### 1. TrustStore is a standalone module

```rust
pub struct TrustStore {
    devices: Vec<TrustedDevice>,
}
```

Clean API: `is_trusted()`, `get()`, `add()`, `touch()`, `forget()`,
`rename()`, `devices()`, `len()`, `is_empty()`. No dependency on
AppState or Document.

### 2. TrustedDevice gains new fields with serde defaults

```rust
pub struct TrustedDevice {
    pub device_id: DeviceId,
    pub device_name: String,
    pub last_seen: u64,
    pub paired_at: u64,
    #[serde(default)]
    pub pairing_method: PairingMethod,
    #[serde(default)]
    pub protocol_version: u32,
}
```

- `pairing_method`: `QrCode`, `Otp`, `Manual` (default: `Otp`)
- `protocol_version`: negotiated during handshake (default: `0`)
- `#[serde(default)]` ensures backward compatibility with existing
  `trusted_devices.json` files that lack these fields.

### 3. SharedTrustStore for concurrent access

```rust
pub type SharedTrustStore = Arc<Mutex<TrustStore>>;
```

Factory functions: `shared_store(devices)`, `empty_store()`. Used by
agent.rs, session manager, and connection monitor.

### 4. Document.devices remains the persistence layer

TrustStore wraps the devices vec but does not own persistence. The
Document model continues to serialize to `document.json` via
DocumentStore. TrustStore is a "view" over the persisted data.

This avoids introducing a new file format while still providing a
clean API for new code.

### 5. State.rs keeps its existing methods (transitional)

`AppState::is_trusted()`, `add_device()`, `touch_device()`, etc.
remain unchanged. They continue to work via direct Document access.
New v0.8 code uses TrustStore directly.

**Transitional plan:** AppState trust methods are compatibility code.
TrustStore is the future single trust authority. The dependency
direction will eventually be:

```
AppState → TrustStore → Document persistence
```

Migration from AppState to TrustStore is deferred to v0.9 or v1.0
when the GUI is updated to use TrustStore directly.

## Consequences

### Positive

- Standalone module, no dependency on AppState
- Clean API for trust operations
- Backward compatible with existing persisted data
- SharedTrustStore enables concurrent access from multiple components
- New fields (pairing_method, protocol_version) enable richer trust metadata

### Negative

- TrustStore and AppState both manage trust (transitional duplication)
- TrustStore does not own persistence (must sync with Document)

**Transitional:** Both authorities exist during v0.8. By v1.0,
AppState trust methods will be removed. TrustStore will be the
sole trust authority.

### Risks

- Deserialization of old JSON without `pairing_method`/`protocol_version`
  defaults to `Otp`/`0`. This is correct for existing paired devices.

## Compliance

- TrustStore MUST provide O(1) lookup by device_id
- TrustedDevice MUST use serde defaults for backward compatibility
- TrustStore MUST NOT own persistence (Document does)
- New v0.8 code MUST use TrustStore, not AppState trust methods
- TrustStore IS the future single trust authority (v1.0 target)
- AppState trust methods ARE transitional compatibility code
