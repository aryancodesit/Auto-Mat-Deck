# ADR-020: Projection Transport and Latest-State Delivery

**Status:** Accepted
**Date:** 2026-07-14

## Context

ADR-014 established `ProjectionPublisher` as the transport boundary. ADR-015
defined lossy latest-state broadcast semantics and mandated that the publisher
retain the latest approved projection for reconnecting consumers. ADR-016
defined the synchronous OS-thread projection model.

Sprint 3 implemented the pipeline up to `LoggingPublisher` (temporary bootstrap).
No concrete transport implementation exists. The `active_profile_state` message
is documented in `protocol.md` but not wired.

v0.4 must bridge the synchronous projection thread to the Tokio-owned WebSocket
connection domain, preserve all v0.3 architectural guarantees, and deliver
projection state to trusted Android clients only.

## Decision Drivers

1. **Latest-state semantics** — the transport must preserve the ADR-015
   lossy model: newest value authoritative, no historical accumulation.
2. **OS-thread → Tokio boundary** — the projection thread is a native
   `std::thread`; it cannot efficiently `await` Tokio I/O.
3. **Synchronous handoff** — `publish()` must not require a Tokio runtime.
4. **Multiple consumers** — concurrent Android clients must each receive
   the latest state independently.
5. **New consumer receives latest** — a newly connected or reconnected
   client must immediately observe the current projection state.
6. **Bounded memory** — no unbounded per-client queues.
7. **Failure isolation** — transport failures must not reach the runtime.

## Considered Alternatives

### A. tokio::sync::watch — SELECTED

| Property | watch |
|---|---|
| Sync handoff from native thread | Yes — `send_replace()` is a sync fn, no runtime required |
| `publish()` must not await | Yes — `send_replace()` completes immediately |
| Bounded state | Yes — exactly one value of type T |
| Historical queue | No — newest replaces oldest |
| Slow consumer accumulation | No — consumer sees `borrow()` / `changed()` — never FIFO |
| Newest state authoritative | Yes — `send_replace()` overwrites |
| Multiple consumers | Yes — `Receiver` is cloneable |
| New receiver sees latest | Yes — `borrow()` reads current value immediately |
| No receivers exists | `send_replace()` still succeeds; channel retains value for future subscribers |
| Blocking | `send_replace()` does not block |

`watch` is the only primitive that matches all ADR-015 latest-state
requirements. It is the transport-level continuation of the semantic choice
made in ADR-015: lossy, latest-state, advisory-notification delivery.

### B. tokio::sync::mpsc (bounded, capacity=1) — REJECTED

- `send()` is async — would require a Tokio runtime on the projection thread
- `blocking_send()` blocks until capacity opens — can block indefinitely
  if a slow consumer hasn't drained the single slot
- FIFO semantics: capacity=1 stores one in-flight message, not the latest
  authoritative state; publisher cannot replace the in-flight message with a
  newer one
- Single-consumer only — cannot support multiple Android clients

### C. tokio::sync::mpsc (unbounded) — REJECTED

- Unbounded per-client FIFO queue re-creates historical projection
  accumulation (contradicts ADR-015 line 86–89)
- If a mobile writer task stalls, the channel grows without limit
- Single-consumer only

### D. tokio::sync::broadcast — REJECTED

- Ring buffer retains history up to capacity — a slow consumer accumulates
  intermediate values rather than converging to latest state
- New subscribers do not see previously sent values — cannot serve
  reconnect-snapshot semantics without retaining the last value externally
- Lag/recovery model conflicts with authoritative-latest-state semantics

### E. Per-client latest-value cell (Mutex<Option<T>> + notify) — REJECTED

- Re-implements watch with a different API
- Requires manual notification wiring for each client
- `watch` provides the same semantics with a tested, maintained Tokio API

## Decision

### Primitive

`tokio::sync::watch` is the OS-thread → Tokio transport handoff primitive.

#### Watch value

The watch channel carries a pre-serialized, immutable transport message:

```
watch::Sender<Option<Arc<str>>>
```

| Variant | Meaning |
|---------|---------|
| `None` | No projection has yet been approved and published |
| `Some(Arc<str>)` | A serialized JSON `active_profile_state` message |

Rationale for pre-serialized `Arc<str>`:
- Serialization runs once on the projection thread, before the handoff
- All subscribers receive the identical string — no per-client serialization
- `Arc<str>` is cheaply cloneable (`O(1)` refcount bump)
- `Option` handles the initial "no projection yet" state without inventing
  a fake projection value

#### Initial state

`watch::Sender::new(None)`

No synthetic or default `active_profile_state` is created. A connection
that reaches `trusted` before any projection has been published will wait
for the first `changed()` notification.

#### Sender ownership

The `watch::Sender` is owned by the `ProjectionTransportPublisher` instance,
which lives inside the `Arc<dyn ProjectionPublisher>` held by the projection
thread. When the projection thread exits, the publisher is dropped, and the
sender is dropped — closing the channel and terminating all receivers.

#### Receiver ownership

Each trusted connection's `handle_connection` task clones a `watch::Receiver`
from the sender. The receiver is owned by that task. When the connection
drops, the receiver is dropped — no cleanup needed.

### Ownership model

| Component | Owner | Thread |
|-----------|-------|--------|
| `watch::Sender` | `ProjectionTransportPublisher` | Projection thread |
| `watch::Receiver` (clone per client) | `handle_connection` task | Tokio runtime (agent-server thread) |
| `ProjectionTransportPublisher` | `Arc<dyn ProjectionPublisher>` | Projection thread (sole ref after spawn) |
| WebSocket sink (`SplitSink`) | `handle_connection` task | Tokio runtime |

No WebSocket sink crosses the thread boundary. The projection thread owns
the `watch::Sender`; connection tasks own `watch::Receiver`s. The handoff is
by value (pre-serialized string) — no shared mutable state.

### Serialization boundary

```
RuntimeProjection
      │
      ▼
ActiveProfileStateMessage (transport DTO, serde::Serialize)
      │
      ▼
serde_json::to_string()
      │
      ▼
Arc<str> → watch::Sender::send_replace()
```

- Serialization occurs once, inside `ProjectionTransportPublisher::publish()`.
- Serialization failure: logged and the projection is dropped. The channel's
  retained value is NOT updated — previous state survives. This matches
  ADR-015 lossy delivery semantics (a failed send is equivalent to a
  transient I/O error).
- The DTO is a separate type from `RuntimeProjection`. Domain fields
  (`context_changed`, `active_profile_changed`) are excluded. Only
  wire-justified fields appear in the DTO.

### Failure behavior

| Failure | Effect | Recovery |
|---------|--------|----------|
| No receivers exist | `send_replace()` succeeds, channel retains value | Value delivered when a receiver subscribes |
| Serialization fails | Log, drop, channel unchanged | Next publish attempt |
| Receiver lag/coalescing | `changed()` returns `true` only when a new value was available since last read — consumer sees latest | Automatic; consumer catches up to latest |
| Sender dropped (shutdown) | Receivers see closed channel — terminate gracefully | No recovery (process shutting down) |
| WebSocket write fails | Log, terminate connection handler (receiver drops, sink drops) | Reconnect on new WebSocket connection |
| Android disconnect | Receiver dropped, no cleanup needed | Reconnect creates new receiver |

## Consequences

### Positive

- Pipeline remains synchronous on the projection thread — no async runtime
  required in the projection layer
- Latest-state semantics extend naturally from `TransitionCell` →
  `PublicationPolicy` → `watch` channel: all three stages use lossy,
  newest-authoritative, bounded-O(1) storage
- New and reconnecting clients receive the latest state immediately via
  `Receiver::borrow()`
- Pre-serialized value avoids N serializations for N subscribers
- `watch` does not accumulate historical projections (unlike `mpsc` or
  `broadcast`)
- No arc mutex registry of WebSocket sinks — the channel is the registry

### Negative

- Pre-serialized `Arc<str>` couples the watch value to the JSON format —
  if the wire format changes, the entire DTO must be re-serialized
- `watch::Receiver` is `!Unpin` — requires `tokio::pin!` in `select!` loops
- Serialization failure drops the projection silently (acceptable under
  ADR-015 lossy policy)

### Neutral

- ADR-014's open question about `&RuntimeProjection` vs owned signature
  is resolved: `publish()` takes a reference, the publisher clones/maps
  internally
- The `Arc<str>` choice is a transport-layer concern — the projection
  domain remains unaware of it

## Compatibility with ADR-014

- ProjectionPublisher trait unchanged in signature
- Domain remains transport-neutral
- `RuntimeProjection` gains no `Serialize` derive
- No transport code calls back into projection or runtime

## Compatibility with ADR-015

- Latest-state semantics preserved: `watch::send_replace` replaces the
  old value with the new one — no history, no queue
- Reconnect semantics: new/cloned `Receiver::borrow()` returns the latest
  retained value immediately
- Lossy delivery: slow consumers converge to latest state; intermediate
  values may be missed
- Publisher lifecycle: publisher is created once, lives for process duration
- Failure isolation: serialization failure does not propagate

## Compatibility with ADR-016

- Projection thread remains synchronous — `publish()` calls
  `send_replace()` without awaiting
- Projection thread never acquires a DesktopRuntime lock
- Shutdown: `watch::Sender::send_replace()` is runtime-independent (Tokio 1.x
  infallible sync operation). The projection thread may outlive the Tokio
  runtime; connection tasks are aborted when the runtime drops — delivery
  loss during shutdown is an accepted failure. `watch::Sender` is dropped
  before or during runtime teardown.
- The latest-value cell and watch channel together form a two-stage lossy
  handoff (TransitionCell → watch) — semantically coherent

## Rejected Designs

### Direct WebSocket sink access from `publish()`

Rejected. Would require sharing `Arc<Mutex<Vec<SplitSink>>>` across threads.
Tokio `SplitSink` is not `Sync`. Projection thread cannot `await` on writes.
A slow client could stall all clients sharing the same lock.

### PublicationPolicy exposed to transport for reconnect

Rejected. `PublicationPolicy.last_emitted` is private. Exposing it would
create a reverse dependency from transport → projection layer, violating
ADR-014's one-way pipeline. The transport hub retains the *approved*
projection after `publish()` is called — this is architecturally distinct
from the policy's internal state.

### `ConnectedClients::close_all()` registry API

Deferred. Not approved without evidence that explicit client lifecycle
management is required. The current fire-and-forget `tokio::spawn` pattern
and the watch channel's receiver-drops-on-disconnect model provide adequate
lifecycle management for v0.4.

## Resolved Questions

1. **`send_replace()` panic / `catch_unwind`?** Resolved. Tokio 1.x
   `Sender::send_replace()` is infallible and requires no Tokio runtime.
   See Tokio 1.52.3 docs: "This method permits sending values even when there
   are no receivers." No panic path exists under the documented ownership
   model (Sender owned by publisher, alive during `publish()`). No
   `catch_unwind` is required or recommended.

2. **Shutdown lifetime coordination?** Resolved within ADR-020. The
   watch channel's Sender is independent of the Tokio runtime;
   `send_replace()` does not require the runtime to be alive. The
   projection thread may therefore legitimately outlive the server
   runtime. Connection tasks are aborted when the runtime drops — delivery
   loss during process shutdown is an accepted failure (see ADR-015).
   No additional lifecycle primitive is required for v0.4.

## Open Questions

1. Should the `ProjectionTransportPublisher` hold an `Arc<Sender>` or own
   the `Sender` directly? The publisher is owned by the projection thread
   via `Arc<dyn ProjectionPublisher>` — this is an implementation detail
   to be resolved during implementation.
2. Should the `Agent` module expose an `Arc<watch::Sender>` that the
   publisher can access, or should the publisher be constructed with the
   sender already wired? The latter is architecturally cleaner — dependency
   injection at construction time.
