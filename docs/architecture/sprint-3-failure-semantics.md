# Sprint 3 — Failure Semantics

**Status:** Draft
**Date:** 2026-07-13

Normative references: Sprint 3 Scope §7–§8, ADR-015 §"Failure behavior",
ADR-016 §"Failure isolation".

---

## 1. Failure taxonomy

| Layer | Failure mode | Scope | Behaviour |
|-------|-------------|-------|-----------|
| Storage write | `Mutex` poisoned by earlier panic | Observer thread | `PoisonError` → fatal process failure |
| Storage write | Contention (brief) | Observer thread | `lock().unwrap()` blocks; no I/O involved, negligible |
| Projection engine | Logic bug (panic) | Projection thread | Thread dies → no further projections → fatal process failure |
| Publication policy | Logic bug (panic) | Projection thread | Same as engine panic |
| Publisher | I/O error (transient) | Projection thread | Silently swallowed; next publication may succeed |
| Publisher | I/O error (permanent) | Projection thread | Silently swallowed; no further deliveries |
| Publisher | Panic | Projection thread | Thread dies → fatal process failure |
| Transport | Disconnect | Publisher thread | Publisher recovers internally or delivery stops |
| Notification | Lost | Synchronization | Next observation retriggers; lossy policy accepts this |
| Observer | Panic | Observer thread | No more transitions → fatal process failure |
| Observer | Win32 API failure | Observer thread | `Err` returned, `successful_observation` returns `None`, runtime state preserved |

## 2. Failure boundaries

```
DesktopRuntime
      │
      │  observer panic → fatal process failure
      │  lock poison → fatal process failure
      ▼
RuntimeTransition
      │
      │  (pure data — cannot fail)
      ▼
ProjectionEngine
      │
      │  panic → fatal process failure
      ▼
PublicationPolicy
      │
      │  panic → fatal process failure
      ▼
Latest-value storage
      │
      │  lock poison → fatal process failure
      ▼
ProjectionPublisher
      │
      │  I/O error → swallowed → delivery lost
      │  panic → fatal process failure
      ▼
Transport
      │
      │  disconnect → no delivery until reconnect
```

Failures at or below `ProjectionPublisher` never propagate into
`DesktopRuntime` or the observer thread. The runtime continues operating.

## 3. Recovery policy

| Scope | Recovery |
|-------|----------|
| Transient I/O error | Automatic on next successful `publish()` call |
| Permanent I/O error | No recovery — latest projection retained in storage, delivered if publisher heals |
| Fatal process failure | Required for any panic at observer, projection, or storage layers; recovery is outside Sprint 3 |
| Lost notification | Implicit recovery on next observation (lossy policy) |

No component implements retry logic, back-off, or circuit breaking in
Sprint 3. The lossy policy makes retry unnecessary: the latest value is
always available for the next successful delivery.

## 4. Observable effects by deployment phase

| Phase | Failure scenario | Observable effect |
|-------|-----------------|-------------------|
| Development | Mock publisher panics | Test fails quickly with clear panic message |
| Development | NullPublisher used | No output — tests verify projection logic only |
| Production | WebSocket disconnect | Device shows stale deck until reconnect |
| Production | WebSocket reconnect | Device receives latest projection on reconnect |
| Production | Observer thread panics | Device shows stale deck; desktop app restarts |
