# ADR-016: Thread Ownership

**Status:** Draft
**Date:** 2026-07-13

## Context

Sprint 2 established two threads: the **context-observer thread** (Win32
polling) and the **main thread** (GUI, tray, server). Runtime state is
shared via `Arc<Mutex<DesktopRuntime>>`.

Sprint 3 introduces the projection pipeline (`ProjectionEngine`,
`PublicationPolicy`, `ProjectionPublisher`). This ADR defines which thread
owns each stage, how locks are acquired and released, and how shutdown is
ordered.

## Decision

### Thread model

Three threads participate in the projection pipeline:

| Thread | Owns | Created |
|--------|------|---------|
| Observer | Win32 polling → `DesktopRuntime::apply_context_observation` | Sprint 2 |
| Projection | `PublicationPolicy` → `ProjectionPublisher` | Sprint 3 |
| Main | GUI, tray, server, `DesktopRuntime` mutations via commands | Sprint 1 |

The observer thread acquires the runtime lock, applies the observation,
produces a `RuntimeTransition`, and writes it to a latest-value storage
cell for the projection thread. The projection thread acquires no runtime
lock.

### Pipeline thread assignment

```
Observer thread
       ↓
  DesktopRuntime::apply_context_observation
       ↓
  RuntimeTransition → latest-value storage
       ↓
Projection thread (new) — wakes on notification
       ↓
  ProjectionEngine           (pure, no lock)
       ↓
  RuntimeProjection
       ↓
  PublicationPolicy          (pure, no lock)
       ↓
  ProjectionPublisher        (may block, no lock on runtime)
```

### Lock acquisition order

1. **Observer thread** acquires `Arc<Mutex<DesktopRuntime>>` for the
   shortest possible duration: just long enough to call
   `apply_context_observation` and produce the `RuntimeTransition`.
2. The `RuntimeTransition` is written to latest-value storage **after the
   runtime lock is released**.
3. **Projection thread** never acquires the runtime lock.
4. **Main thread** acquires the runtime lock for command dispatch and
   reconciliation (Sprint 2 pattern unchanged).

Lock order across threads:
- Observer: lock → observe → unlock → store → notify
- Main: lock → dispatch → reconcile → unlock
- Projection: (no lock)

No thread holds multiple runtime locks simultaneously. No lock inversion
is possible because only one lock (`Mutex<DesktopRuntime>`) exists.

### Projection thread creation and lifecycle

The projection thread is spawned during `main()` alongside the observer
thread. It runs an event loop:

```
loop:
    wait for notification
    read latest RuntimeTransition from storage
    let projection = project(&transition)    // pure
    let policy = should_publish(&projection) // pure
    if policy.should_publish():
        publisher.publish(&projection)       // may block, no lock held
```

Notification wakes the projection thread when a new transition is available.
The observer writes the transition before notifying, so the reader always
sees the latest value.

Shutdown: a separate shutdown flag or channel-drop signal causes the wait
to terminate. No additional signal is required beyond the existing shutdown
mechanism (see Sprint 2 observer shutdown pattern).

### Synchronization requirements

The observer→projection handoff must satisfy:

| Requirement | Rationale |
|-------------|-----------|
| **Bounded memory** | Unbounded queues risk exhaustion if publisher stalls. O(1) storage. |
| **Newest value authoritative** | Older unpublished transitions are discardable. Lossy delivery. |
| **Observer never blocks** | Win32 polling must not stall on delivery. Non-blocking write. |
| **Projection thread may lag** | Publisher I/O must not affect observer or runtime. Decoupled threads. |
| **Shutdown signal** | Projection thread must exit cleanly when the application shuts down. |

These requirements are satisfied by a **latest-value synchronization
mechanism** with three conceptual parts:

| Part | Role |
|------|------|
| **Bounded storage** | Holds at most one `RuntimeTransition` |
| **Mutual exclusion** | Coordinates reader/writer access |
| **Notification** | Wakes the projection thread when a new value is available |

The observer **overwrites** storage without waiting. If the projection
thread has not yet read the previous value, it is silently dropped. This
is **lossy by design**.

**Notifications are advisory, not authoritative.** The stored transition
is the source of truth. A notification indicates *something may have
changed*; the projection thread must read storage to discover what.
This means a lost notification is safe: the next successful notification
will cause the reader to observe the latest value. The system never
assumes "notification received" implies "new transition available" —
the read confirms availability.

The concrete primitives (e.g. `Mutex + Condvar`, `watch` channel, atomic
flag with spin, Windows event) are an **implementation decision** — any
mechanism that satisfies the five requirements above is acceptable. The
architecture mandates the semantics, not the API.

### Whether publication is synchronous

Publication from the projection thread is synchronous — `publish` blocks
the projection thread until delivery completes. The projection thread is
not the observer thread, so blocking does not affect Win32 polling.

If the publisher blocks indefinitely, the storage cell is overwritten on
each new observation and the projection thread processes only the latest
value when it wakes. Runtime operation is unaffected.

### Whether publisher may block

Yes. The publisher may block on I/O (e.g. WebSocket write). Because the
publisher holds no runtime lock, blocking cannot poison runtime state or
prevent context observations from being processed.

If blocking becomes problematic (projection thread starves), the publisher
can be made asynchronous in a future sprint without changing the pipeline
architecture.

### Shutdown ordering

1. Main thread signals shutdown flag (or drops notification primitive).
2. Projection thread wakes, sees shutdown signal, exits.
3. Main thread joins projection thread.
4. Main thread drops `shared_runtime.clone()` (observer's reference).
5. Observer thread exits when shutdown watch channel closes.
6. Main thread joins observer thread.

This ensures the projection thread exits before the observer thread,
preventing any attempt to store a transition after the consumer is gone.

### Failure isolation

| Failure | Effect | Recovery |
|---------|--------|----------|
| Publisher blocks | Projection thread lags, transitions overwritten | Auto when publisher unblocks |
| Publisher panics | Projection thread dies, storage ownerless | Process restart (no recovery in scope) |
| Notification lost | Projection thread misses wake-up | Next observation triggers another notification |
| Observer panics | No more transitions | Process restart |
| Runtime lock poisoned | All threads see `PoisonError` | Process restart |

The lossy latest-value semantics ensure no failure in the publisher can
block or corrupt the observer or runtime.

## Race-condition analysis

| Scenario | Analysis |
|----------|----------|
| Observer stores transition while main thread holds lock | Runtime lock is released before storage write. No race. |
| Projection thread reads transition while observer stores another | Storage mutex arbitrates. Reader sees either old or new value — both are valid under lossy semantics. |
| Publisher blocks while observer polls Win32 | Different threads. No shared state. No race. |
| Observer and main contend for runtime lock | Standard `Mutex` arbitration. No deadlock. |
| Notification arrives before observer finishes write | Observer writes before notifying (release-acquire ordering). Reader sees latest value. |

## Deadlock analysis

| Scenario | Outcome |
|----------|---------|
| Observer locks runtime, tries to write to storage | Runtime lock released before storage write. No deadlock. |
| Publisher blocks, observer overwrites storage | Write is non-blocking. Observer never waits. No deadlock. |
| Main thread joins projection thread while holding runtime lock | Projection thread never acquires runtime lock. No deadlock. |

No lock inversion or nested lock acquisition exists in the projection
pipeline.

## Alternatives Considered

### ProjectionEngine on observer thread

**Rejected.** Running projection on the observer thread would increase
lock-hold time and couple projection computation to Win32 polling latency.
Moving projection to a dedicated thread keeps the observer loop minimal.

### Queue-based channel (sync_channel, crossbeam, etc.)

**Rejected.** Any FIFO queue introduces an architectural mismatch: the
observer cannot discard the oldest queued item when a newer one arrives.
`std::sync::mpsc::sync_channel(1)` + `try_send` drops the *newest* (the
send fails), not the oldest. A latest-value cell is the correct primitive
for lossy delivery.

### Unbounded queue

**Rejected.** An unbounded queue would grow without limit if the publisher
stalls, eventually exhausting memory. Bounded O(1) storage matches the
lossy policy.

### Direct call (observer calls publisher)

**Rejected.** Without decoupled storage, the observer would need to call
the publisher directly, coupling Win32 polling to I/O delivery latency.
The latest-value cell with notification decouples the threads.

### Tokio broadcast channel

**Deferred.** A `tokio::sync::broadcast` channel would support multiple
consumers natively. Sprint 3 has a single consumer (the projection thread).
Multi-consumer broadcast can be introduced when additional subscribers
(Android, logging) are added. Broadcast also preserves *all* values
(not just latest), which conflicts with lossy semantics unless the
consumer actively skips.

## Consequences

Positive:
- Observer never blocks on delivery — Win32 polling is isolated.
- Projection thread holds no runtime lock — cannot poison runtime.
- Latest-value storage enforces lossy semantics with bounded O(1) memory.
- Shutdown is deterministic: signal → projection exits → observer exits.
- All thread interactions are synchronous primitives — no async runtime
  required.

Negative:
- A dedicated projection thread adds one OS thread (~64 KB stack).
- Rapid context changes may drop transitions before they are projected.
  This matches the lossy policy.
- A notification can be lost if the primitive does not guarantee delivery.
  This is acceptable because Sprint 3 guarantees only eventual publication
  of the latest state, not publication of every transition. The stored
  transition is authoritative; the next successful notification (triggered
  by the next observation) will cause the reader to pick up the latest
  value. Lossy delivery makes this an intentional property, not a bug.

Neutral:
- If multiple consumers are needed in the future (WebSocket + logging),
  the latest-value cell can be extended with a reader list.
