# Sprint 3 — Active Profile Broadcast & Deck Synchronization

## 1. Objective

Introduce a runtime projection pipeline that converts `RuntimeTransition` into a
transport-neutral representation suitable for external consumers, while
preserving all Sprint 2 architectural guarantees.

## 2. In Scope

- `RuntimeProjection` data model
- `ProjectionEngine` — pure function mapping `RuntimeTransition → RuntimeProjection`
- `ProjectionPublisher` — abstract interface for transport emission
- Synchronization pipeline wiring `DesktopRuntime → ProjectionEngine → ProjectionPublisher`
- Full unit-test suite for projection (zero Win32/networking/UI dependencies)
- Architecture documentation (Scope, ADR, sequence diagrams, thread model,
  failure semantics, transport contract, test matrix)

## 3. Out of Scope

- Android implementation (client)
- Mobile or desktop UI changes
- Deck rendering or hardware sync
- WebSocket protocol design or evolution
- Bidirectional communication (command channel)
- Remote commands or cloud synchronization
- Persistent projection history or replay
- Any modification to `ForegroundObserver`, `DesktopRuntime`, `apply_context_observation`,
  `successful_observation`, or `RuntimeTransition`
- Win32 API interaction of any kind
- Temporal debounce or stability counters (deferred from Sprint 2)

## 4. Inputs

| Input | Source | Notes |
|-------|--------|-------|
| `RuntimeTransition` | `DesktopRuntime` | The sole input. No observer callbacks, no Win32 handles, no UI events. |

## 5. Outputs

| Output | Type | Notes |
|--------|------|-------|
| `RuntimeProjection` | Transport-neutral struct | A derived value — must not own or reference `DesktopRuntime`. No serialization format at this layer. |

## 6. Architectural Constraints

Dependency direction is **one-way only** — no layer calls back "up" the chain:

```
ForegroundObserver
       ↓
  successful_observation
       ↓
  DesktopRuntime
       ↓
  RuntimeTransition
       ↓
  ProjectionEngine
       ↓
  RuntimeProjection
       ↓
  ProjectionPublisher
       ↓
  Transport (future)
```

Rules:
- `observer.rs` must never import networking, serialization, or UI modules.
- `DesktopRuntime` must never serialize JSON or know about transports.
- `ProjectionEngine` must be a pure function — no I/O, no side effects, no state.
- `ProjectionPublisher` is an interface; the domain layer references the
  interface only. No concrete transport types enter the domain.
- WebSocket/HTTP/Android code must never resolve profiles or inspect
  `DesktopRuntime` internals.
- `ProjectionPublisher` must not modify, enrich, or derive projection data.
  Its responsibility is delivery only.
- `RuntimeProjection` is a derived value and must not own or reference
  `DesktopRuntime`.

## 7. Architectural Invariants

- `RuntimeTransition` has exactly one consumer inside Sprint 3 (the
  `ProjectionEngine`).
- Projection never feeds back into `DesktopRuntime` — no mutation of runtime
  state from the projection layer.
- Projection generation is deterministic: identical `RuntimeTransition` inputs
  produce identical `RuntimeProjection` outputs.
- Projection generation performs no I/O, no side effects, and holds no state.
- Publisher failure never mutates runtime state — the projection layer is
  stateless and the publisher is downstream only.

## 8. Non-goals

- Profile resolution or context rule evaluation
- Win32 interaction of any kind
- JSON generation or wire format design
- Socket lifecycle or reconnection logic
- Retry or buffering policy implementation
- Concurrent access safety beyond `Arc<Mutex<…>>` established in Sprint 2
- Real-time latency guarantees

## 9. Success Criteria

- `RuntimeProjection` fully unit-testable without Win32, networking, or UI
- `RuntimeTransition` is the sole input to `ProjectionEngine`
- Zero Win32 dependencies in projection or publisher layers
- Zero networking dependencies in projection layer
- Zero UI dependencies in projection or publisher layers
- All existing Sprint 2 tests continue to pass unmodified
- `ProjectionPublisher` interface contains no transport-specific types

## 10. Back-pressure Policy

**Lossy delivery** — only the latest `RuntimeProjection` is meaningful.
Consumers care about the current active profile, not every intermediate
transition. Only the newest projection is considered authoritative; older
unpublished projections may be discarded. If a consumer is slow or
disconnected, it receives only the most recent state upon reconnection.

This is the initial policy; evidence from real-world usage may justify
upgrading to coalesced delivery later.

## 11. Open Questions

- Should `RuntimeProjection` expose the full `RuntimeTransition` fields, or
  should it be a subset (e.g. only `active_profile_id`)?
- Should `ProjectionPublisher::publish` be blocking or async?
- Who owns the publisher lifecycle — the projection pipeline or an external
  orchestrator?
- Should the pipeline run on the observer thread, the main thread, or a
  dedicated projection thread?
- Is back-pressure signalling required (e.g. slow consumer detection)?

## 12. Sprint Acceptance Criteria

- [ ] `RuntimeProjection` model defined and reviewed
- [ ] `ProjectionEngine` implemented as a pure function
- [ ] `ProjectionPublisher` trait defined with no transport types
- [ ] Pipeline wired in `main.rs` (or equivalent orchestration)
- [ ] Full test matrix passing (see Test Matrix document)
- [ ] All Sprint 2 tests pass unmodified
- [ ] `ProjectionEngine` produces identical output for identical `RuntimeTransition` inputs
- [ ] `cargo test` — all tests passing
- [ ] `cargo build --release` — clean build
- [ ] Sprint 3 ADR approved
- [ ] Sequence diagrams approved
- [ ] Thread ownership model approved
- [ ] This document remains the gatekeeper: no downstream doc overrides scope
