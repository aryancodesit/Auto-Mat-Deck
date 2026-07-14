# ADR-014: Projection Architecture

**Status:** Draft
**Date:** 2026-07-13

## Context

Sprint 2 established `DesktopRuntime` as the orchestration boundary. Context
observations flow through `ForegroundObserver → successful_observation →
DesktopRuntime → RuntimeTransition`. The runtime transition tells us *what
changed*.

Sprint 3 needs to answer: *who else learns about it?*

A direct path from `DesktopRuntime` to WebSocket (or any transport) would
introduce coupling between runtime state and network delivery. The runtime
would need to know about serialization, socket lifecycle, reconnection, and
back-pressure — concerns that are orthogonal to context resolution.

A projection layer solves this by converting `RuntimeTransition` into a
transport-neutral representation. Transport code consumes this representation
without importing runtime types.

## Decision

### Projection layer exists

A dedicated `ProjectionEngine` converts `RuntimeTransition` to
`RuntimeProjection`. This layer is a pure function — no I/O, no side effects,
no state, no knowledge of transports.

### RuntimeTransition is the sole input

The projection engine accepts `RuntimeTransition` only. It does not receive
`DesktopRuntime`, `ProfileRuntime`, `ContextSnapshot`, or any Win32 type.

Rationale: constraining the input prevents the projection layer from
acquiring runtime dependencies. If the engine needed more data in the future,
`RuntimeTransition` would be extended — the engine itself stays pure.

### RuntimeProjection is immutable

`RuntimeProjection` is a `struct` (or equivalent) with only public read access.
Once constructed by `ProjectionEngine`, it is never mutated. Consumers clone
or borrow as needed.

### Projection is a pure function

```rust
fn project(transition: &RuntimeTransition) -> RuntimeProjection
```

Same input always produces same output. No interior mutability, no caching, no
 I/O. This makes the engine fully unit-testable without mocks, timers, or
async runtimes.

### Publishers are abstract interfaces

`ProjectionPublisher` is a trait with one method:

```rust
trait ProjectionPublisher {
    fn publish(&self, projection: &RuntimeProjection);
}
```

No transport-specific types appear in this trait. The domain layer references
the trait only. Concrete implementations (WebSocket, logging, mock) live
outside the projection module.

**Ownership note:** The `&RuntimeProjection` borrow is intentionally
undecided at this layer. If transport proves to be asynchronous, the
signature may become `fn publish(&self, projection: RuntimeProjection)` or
`fn publish(&self, projection: Arc<RuntimeProjection>)`. The ownership model
will be resolved when the transport ADR is drafted, not before.

### Transport is downstream only

The pipeline is unidirectional:

```
DesktopRuntime → RuntimeTransition → ProjectionEngine → RuntimeProjection → ProjectionPublisher → Transport
```

No transport code calls back into projection, runtime, or observer layers.
No projection code calls back into runtime. The observer has no knowledge of
anything downstream.

## Architectural Invariants

- Projection data must be derived exclusively through `ProjectionEngine`.
  (Multiple components may observe `RuntimeTransition` for orthogonal
  concerns such as logging or metrics; none may derive projection data.)
- Projection never feeds back into `DesktopRuntime` — no mutation of runtime
  state from the projection layer.
- Projection generation is deterministic: identical `RuntimeTransition` inputs
  produce identical `RuntimeProjection` outputs.
- Projection generation performs no I/O, no side effects, and holds no state.
- Publisher failure never mutates runtime state — the projection layer is
  stateless and the publisher is downstream only.
- `ProjectionPublisher` must not modify, enrich, or derive projection data.
  Its responsibility is delivery only.
- `RuntimeProjection` is a derived value and must not own or reference
  `DesktopRuntime`.

## Alternatives Considered

### Direct DesktopRuntime → WebSocket

**Rejected.** Couples runtime to networking. Violates Sprint 2 separation.
Would require mocking WebSockets in runtime tests.

### Observer → Transport direct emission

**Rejected.** Observer already has its own responsibility (Win32 polling).
Adding transport emission would make the observer responsible for delivery
semantics, back-pressure, and serialization.

### RuntimeTransition embedded in transport messages

**Rejected.** `RuntimeTransition` is an internal domain model.
`RuntimeProjection` is an external contract. Those should evolve
independently. Embedding the internal model in transport messages would
couple wire format changes to domain refactors and vice versa.

### Publisher returning Result for back-pressure

**Deferred.** Returning `Result` from `publish` would force the caller to
handle delivery failures, which implies retry logic or queuing. Sprint 3
starts with lossy delivery (drop on failure). A fallible `publish` can be
added when evidence justifies it.

## Consequences

Positive:
- Projection layer is pure and testable — no mocks, no async, no networking.
- Transport can be replaced without touching domain code.
- Runtime remains unaware of downstream consumers.
- New projection fields can be added by extending `RuntimeProjection` without
  changing the engine's contract.
- Projection contracts become versionable independently of runtime internals,
  valuable when Android and Desktop evolve separately.

Negative:
- An extra transformation step adds a small amount of boilerplate.
- Field duplication between `RuntimeTransition` and `RuntimeProjection` is
  possible if the models diverge without discipline.

Neutral:
- The pipeline introduces an explicit projection boundary that must be
  maintained during future sprints. ADR review should catch violations.
