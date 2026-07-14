# Sprint 3 — Transport Contract

**Status:** Draft
**Date:** 2026-07-13

Normative references: Sprint 3 Scope §5–§6, ADR-014, ADR-015, ADR-016.

---

## 1. Pipeline position

```
ProjectionEngine → PublicationPolicy → latest-value sync → ProjectionPublisher
```

`ProjectionPublisher` is the final stage before transport. It receives
projections already filtered by `PublicationPolicy`. It must not duplicate
that filtering.

## 2. The publisher boundary

`ProjectionPublisher` is the only interface between the projection pipeline
and transport. It is defined in the domain layer. Transport code references
it externally.

```rust
/// A publisher of runtime projections.
///
/// Implementations deliver projections to external consumers.
/// Must not modify, enrich, or derive projection data.
/// Must not reference DesktopRuntime, ProfileRuntime, or any Win32 type.
trait ProjectionPublisher: Send + Sync {
    /// Deliver a projection to all subscribed consumers.
    ///
    /// May block (the projection thread tolerates blocking).
    /// Failure is silently swallowed at the publication boundary.
    /// Must not propagate panics or errors to the caller.
    fn publish(&self, projection: &RuntimeProjection);
}
```

## 3. Contract

| Concern | Guarantee |
|---------|-----------|
| Input | `RuntimeProjection` — transport-neutral, immutable, derived |
| Output | Side effect only (e.g. socket write, log line) |
| Modification | Must not modify, enrich, or derive projection data |
| Runtime access | Must not reference `DesktopRuntime`, `ProfileRuntime`, or any Win32 type |
| Synchronization | `Send + Sync` — may be called from the projection thread only |
| Blocking | Allowed — projection thread is dedicated |
| Failure | Silently swallowed — must not propagate to caller |
| Ordering | Called in FIFO order of `ProjectionEngine` output (per ADR-016) |
| Filtering | Must not filter or suppress — `PublicationPolicy` handles that |
| Lifecycle | Created at startup, lives for process duration |

## 4. Non-contract (explicitly not guaranteed)

- Exactly-once delivery
- In-order delivery across async publisher restarts
- Acknowledgment or confirmation
- Consumer count or identity
- Replay of missed projections

## 5. Examples of valid implementations

| Implementation | Responsibility |
|----------------|---------------|
| `LoggingPublisher` | Writes projection to `tracing::info!` for debugging |
| `WebSocketPublisher` | Serializes projection and writes to connected WebSocket sessions |
| `NullPublisher` | Discards all projections — useful for testing or when no transport is needed |
| `FanOutPublisher` | Delegates to multiple inner publishers |

## 6. What transport must NOT do

- Resolve profiles or context rules
- Access `DesktopRuntime` or `ProfileRuntime`
- Import Win32 symbols
- Represent runtime types in wire format (use `RuntimeProjection` fields)
- Suppress or filter projections (that is `PublicationPolicy`'s role)
- Retry failed deliveries (lossy policy — latest projection replaces)
- Block the observer or runtime threads
