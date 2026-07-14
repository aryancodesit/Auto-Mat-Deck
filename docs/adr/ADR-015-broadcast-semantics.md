# ADR-015: Broadcast Semantics

**Status:** Draft
**Date:** 2026-07-13

## Context

ADR-014 defines `ProjectionEngine` as a pure function converting
`RuntimeTransition → RuntimeProjection`, and `ProjectionPublisher` as an
abstract delivery interface.

This ADR defines *when* and *under what conditions* a projection is published,
and introduces an explicit **Publication Policy** layer between the engine and
the publisher. Without explicit broadcast semantics, the pipeline risks
emitting redundant, out-of-order, or meaningless projections — and consumers
cannot rely on the stream's guarantees.

## Decision

### Publication trigger

Publication occurs immediately after `ProjectionEngine` produces a
`RuntimeProjection`. There is no debounce, batching, or scheduling layer
between the engine and the publisher.

The logical flow is:

```
RuntimeTransition
      ↓
ProjectionEngine        → compute
      ↓
RuntimeProjection
      ↓
PublicationPolicy       → decide whether to emit
      ↓
ProjectionPublisher     → deliver
```

`ProjectionEngine` is responsible for computation. `PublicationPolicy` is
responsible for the suppression decision. `ProjectionPublisher` is
responsible for delivery. No layer performs another's role.

Rationale: the engine is a pure function with no I/O — it cannot block. Any
debounce or batching belongs in the transport layer, where latency
requirements are transport-specific.

### Which transitions generate broadcasts

Every `RuntimeTransition` is projected — `ProjectionEngine` runs on every
transition regardless of `context_changed`. Whether every projection is
published is governed separately by `PublicationPolicy`.

Rationale: the engine is a pure function with no I/O; running it on every
transition has negligible cost. Deciding which projections to publish is
a policy concern, not a computation concern.

### Identical projection suppression

`PublicationPolicy` may suppress emission of a projection that is
semantically identical to the previously emitted projection. "Semantically
identical" is defined by `RuntimeProjection` equality (`PartialEq`).

This is an optimization, not a guarantee. Consumers must not depend on
receiving or not receiving identical projections.

Rationale: the lossy back-pressure policy already means consumers may
miss intermediate projections. Suppressing identical projections reduces
unnecessary work without weakening the contract. Suppression is a
`PublicationPolicy` responsibility — the publisher itself never inspects
or filters projections.

### Ordering guarantees

Projection ordering is preserved between `ProjectionEngine` output and
`PublicationPolicy` decisions. The pipeline is single-threaded at the
projection and policy stages (see Thread Ownership ADR), so natural FIFO
ordering holds through publication.

If the publisher is asynchronous (future Sprint), ordering across concurrent
deliveries is a transport concern, not a projection concern. The projection
pipeline itself never reorders.

### Lossy semantics

When a consumer is slow or unavailable, unpublished projections are
discarded in favour of the newest projection. The publisher retains only
the latest projection; when a new projection arrives, the old one is
replaced.

This matches the Sprint 3 scope back-pressure policy:
"Only the newest projection is considered authoritative; older unpublished
projections may be discarded."

### Consumer reconnect semantics

When a consumer reconnects, the publisher emits the latest retained
projection. The consumer does not receive a replay of missed projections.

Rationale: the system is authoritative for *current* state only. Replay
history would require persistent storage and ordering guarantees that are
out of scope for Sprint 3.

### Failure behavior

Publisher failures terminate at the **publication boundary** and must never
propagate into `ProjectionEngine`, `PublicationPolicy`, or `DesktopRuntime`.
The pipeline continues producing projections; the latest projection is
retained and delivered upon reconnection.

Rationale: the projection pipeline is downstream of `DesktopRuntime` and
must never block or poison runtime operation. A failed publication is
equivalent to a slow consumer — the latest state is retained, nothing
more.

### Publisher lifecycle

The publisher is created once at application startup and lives for the
duration of the process. It is not recreated on failure; a failed publisher
remains alive and may recover internally (e.g. WebSocket reconnect).

If the publisher cannot recover, no further deliveries occur for its
subscribed consumers. The pipeline still produces projections; they are
retained as the latest state for any future consumer that binds to a
working publisher.

## Rationale summary

| Decision | Rationale |
|----------|-----------|
| Publish immediately | Engine is pure, cannot block |
| Every transition is projected | Engine is pure — negligible cost |
| Suppress identical | PublicationPolicy decision, not publisher |
| FIFO from engine | Single-threaded, natural ordering |
| Lossy on slow consumer | Back-pressure policy |
| Latest on reconnect | No replay history |
| Swallow publish failure | Never block runtime |
| Static lifecycle | Simplest correct model |

## Alternatives Considered

### Batch publications on a timer

**Rejected.** Timer-based batching introduces latency for no benefit when
the engine is already the bottleneck-free pure function. Batching can be
added in the transport layer if throughput measurements justify it.

### Suppress all unchanged projections

**Rejected.** A projection with `active_profile_id == None` after a context
change is meaningful — it signals "no matching profile." Downstream
consumers should see this even if the previous projection was also `None`.
Suppression by semantic identity (rather than by field-level diff) leaves
this decision to the transport.

### Guarantee delivery exactly once

**Rejected.** Exactly-once delivery requires persistent acknowledgement
tracking, a delivery log, and retry state machine — none of which are
justified for current-state broadcast. The lossy model is correct for the
Sprint 3 use case (deck showing current active profile).

### Suppression logic inside ProjectionPublisher

**Rejected.** Embedding suppression logic in `ProjectionPublisher` would
make the publisher responsible for policy decisions. The publisher's only
responsibility is delivery. Suppression belongs in `PublicationPolicy`,
which can be tested, replaced, or extended independently of any transport.

### Queue projections on publisher failure

**Deferred.** Queuing failed publications would require a bounded or
unbounded buffer, back-pressure signalling, and a drain policy. The lossy
model is simpler and sufficient until evidence proves otherwise.

## Consequences

Positive:
- Pipeline is simple and predictable: produce, retain, publish.
- Runtime is never blocked by delivery.
- Consumers have clear expectations: latest state on connect, lossy in
  between.

Negative:
- Transient failures may cause silent data loss if consumers rely on
  every transition. Mitigation: the scope document chose lossy delivery.
- Suppression by equality may mask bugs where an identical-but-meaningful
  projection is silently dropped. Mitigation: suppression is an opt-in
  transport optimization; the projection layer never skips production.

Neutral:
- Reconnect semantics (latest projection only) make it impossible to
  reconstruct state history from the broadcast stream. This is by design.
