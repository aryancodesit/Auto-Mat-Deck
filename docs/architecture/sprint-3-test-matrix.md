# Sprint 3 — Test Matrix

**Status:** Draft
**Date:** 2026-07-13

Normative references: Sprint 3 Scope, ADR-014, ADR-015, ADR-016.

---

## 1. Architectural invariant tests

These tests verify properties that must hold regardless of the concrete
implementation of any layer.

| # | Test | Invariant | Type |
|---|------|-----------|------|
| 1 | Identical RuntimeTransition → identical RuntimeProjection | ProjectionEngine is deterministic | Determinism |
| 2 | PublicationPolicy suppresses duplicate | `projection == last_emitted` → `false` | Dedup |
| 3 | Overwrite drops prior value | Write T1, write T2, read → T2, not T1 | Newest authoritative |
| 4 | Notification is advisory | Reader can read without notification; writer can write without consumer | ADR-016 invariant |
| 5 | Publisher failure does not affect runtime | Error swallowed → next publish succeeds | Failure isolation |
| 6 | Shutdown: projection exits before observer | Signal → projection thread terminates → observer thread terminates | ADR-016 ordering |
| 7 | Publisher cannot access DesktopRuntime | Trait has no runtime types | Layer isolation |
| 8 | No Win32 in projection or policy modules | Compiles on all platforms | Platform isolation |

## 2. Implementation verification tests

These tests confirm that a specific implementation matches its contract.
They may reference concrete types or primitives.

### ProjectionEngine

| # | Test | Behaviour | Notes |
|---|------|-----------|-------|
| 9 | Context changed: true → projection reflects | `context_changed` field equals input | Field passthrough |
| 10 | Context changed: false → projection reflects | Same | Field passthrough |
| 11 | Profile changed: true → projection reflects | `active_profile_changed` field equals input | Field passthrough |
| 12 | Profile changed: false → projection reflects | Same | Field passthrough |
| 13 | Previous profile maps correctly | `previous_profile_id` copied to projection | Field mapping |
| 14 | Current profile maps correctly | `active_profile_id` copied to projection | Field mapping |
| 15 | None profiles handled | Both fields `None` → projection fields `None` | Null safety |
| 16 | New field requires only engine update | Adding to `RuntimeProjection` compiles without changing callers | Extensibility |

### PublicationPolicy

| # | Test | Behaviour | Notes |
|---|------|-----------|-------|
| 17 | First projection always published | No prior → `true` | Initial state |
| 18 | Different projection published | `projection != last_emitted` → `true` | Change detection |
| 19 | Policy is stateless across reset | Recreate policy clears `last_emitted` | Determinism |
| 20 | Policy does not run engine | Engine runs before policy, regardless of outcome | Separation |

### Latest-value synchronization

| # | Test | Behaviour | Notes |
|---|------|-----------|-------|
| 21 | Store then read returns same value | Single writer → reader sees written value | Correctness |
| 22 | Read on empty returns None | No write → reader sees `None` | Null safety |
| 23 | Multiple writes, single read | Ten writes, one read → tenth value | Newest authoritative |

### Integration (threaded, mock publisher)

| # | Test | Behaviour | Notes |
|---|------|-----------|-------|
| 24 | Full pipeline: unique projection → one publish | `MockPublisher` called once per unique | End-to-end |
| 25 | Full pipeline: dedup suppresses publish | Identical transitions → one call | End-to-end |
| 26 | Fast transitions drop intermediate | 5 rapid writes, slow reader → ≤5 reads | Lossy policy |
| 27 | Publisher failure is isolated | Fail once, succeed next → second delivers | Failure isolation |

### Compile-time guards

| # | Test | Behaviour | Notes |
|---|------|-----------|-------|
| 28 | Wrong type rejected by ProjectionEngine | Compile error | Type safety |
| 29 | Policy receives immutable projection | `&RuntimeProjection` — cannot mutate | Immutability |

## 3. Coverage summary

| Area | Tests | Notes |
|------|-------|-------|
| Architectural invariants | 8 | Implementation-independent |
| ProjectionEngine verification | 8 | |
| PublicationPolicy verification | 4 | |
| Synchronization verification | 3 | |
| Integration | 4 | Threaded, mock publisher |
| Compile-time guards | 2 | |
| **Total** | **29** | |
