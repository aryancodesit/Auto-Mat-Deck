# ADR-013: Runtime Context Transition Semantics

**Status:** Accepted
**Date:** 2026-07-13

## Context

v0.3 introduces `ProfileRuntime` with `latest_context` and
`active_profile_id`. The context observer reports foreground process changes,
but the runtime must determine what changed and what action (if any) is
required.

The question is: how does the runtime represent a context observation
transition, and what are the rules for updating runtime state?

## Decision

### Normalized equality boundary

Context observations are compared using `normalize_process_name` (trim,
lowercase). Two observations whose normalized process names are equal are
deduplicated — no transition is emitted.

The observer returns the basename as observed by the OS (e.g. `"Code.exe"`).
The comparison uses the normalized form. The stored `latest_context` retains
the observed (non-normalized) basename.

Rationale: the observed name is the truth from the OS. Normalization is a
domain comparison concern, not a storage concern. If the observer format
changes in the future, normalization absorbs the difference.

### `latest_context` retention policy

`latest_context` is updated only when a meaningful context change is
detected (normalized comparison produced a different result).

| Observation | `latest_context` after |
|---|---|
| Initial (`None` previously) | Updated to observation |
| Normalized equivalent to previous | **Unchanged** |
| Normalized different from previous | Updated to new observation |
| `Err` (observation failed) | **Unchanged** |

### Observation error does not mutate runtime

When the observer returns `Err`, the polling worker does not call
`apply_context_observation`. `latest_context` and `active_profile_id` are
not touched. This prevents transient Win32 failures (access denied, process
exit race) from erasing retained context.

### Structured RuntimeTransition over boolean

A boolean return from `observe_context` is rejected because it cannot
distinguish between:

A. Context unchanged.
B. Context changed, active profile unchanged.
C. Context changed, active profile A → B.
D. Context changed, active profile None → A.
E. Context changed, active profile A → None.

The structured representation:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeTransition {
    pub context_changed: bool,
    pub previous_profile_id: Option<ProfileId>,
    pub active_profile_id: Option<ProfileId>,
}

impl RuntimeTransition {
    pub fn active_profile_changed(&self) -> bool {
        self.previous_profile_id != self.active_profile_id
    }
}
```

### Context/profile dual change

A single observation can change both context and active profile. The
structured `RuntimeTransition` represents both dimensions simultaneously.
A mutually-exclusive enum was rejected because it cannot represent both
changing in the same transition.

### Why context_changed is needed

`active_profile_changed()` covers Sprint 3 projection (when to send
`active_profile_state`). But `context_changed` is independently useful for:

- Diagnostics/logging — distinguish "observation processed but nothing
  changed" from "no observation occurred."
- Future features that react to context changes independently of profile
  changes (e.g., telemetry, analytics).
- Transparent testing — assertions can verify both dimensions independently.

### Why not ContextSnapshot in the transition

The projection layer (Sprint 3, ADR-011) only consumes the resulting
`active_profile_id` to construct `ActiveProfileState`. Copying
`ContextSnapshot` into the transition would expose observer internals to the
projection layer unnecessarily and create a coupling between the observer
output format and the transition type.

### Why previous_profile_id

`active_profile_changed()` detects a profile transition. This is the trigger
for Sprint 3's `active_profile_state` push condition. Without
`previous_profile_id`, the projection layer would need to cache the previous
`active_profile_id` itself to detect changes — duplicating state that the
runtime transition already captures.

### Manual mode context retention

In `SelectionMode::Manual(pid)`, the observer still runs and
`apply_context_observation` still updates `latest_context`. The context is
recorded even though `resolve_active_profile` is not called in Manual mode.

This ensures that when the user switches back to `Automatic`, the latest
context is immediately available for resolution with no 200 ms delay.

### Stale Manual deletion self-healing

When a `Manual(pid)` becomes stale because the Profile was deleted:

1. `ProfileRuntime::reconcile` (Sprint 1, `model.rs:197`) detects the stale
   `Manual` and transitions to `Automatic`.
2. It then resolves `active_profile_id` using the retained `latest_context`.
3. The retained context may match a rule for a different Profile.

This self-healing works because `latest_context` is always updated regardless
of `SelectionMode` — the retained context is the last successfully observed
foreground process, not the last automatic-resolution input.

### Transport-neutral transition reporting

`RuntimeTransition` is a domain type. It does not contain WebSocket handles,
channels, or any transport mechanism. The polling worker examines the
transition outside the `SharedRuntime` lock and decides whether to trigger
Sprint 3 projection. This keeps projection concerns out of the runtime.

### Sprint 3 projection consumes active-profile changes only

The existing ADR-011 push conditions specify:

1. Initial authorization.
2. Active Profile changes.
3. SelectionMode changes.

All three are covered by `RuntimeTransition::active_profile_changed()` plus
explicit SelectionMode change events. The projection layer does not need
`context_changed`.

## Alternatives considered

### Boolean return

Rejected. Five cases collapsed into `true`/`false` loses information the
projection layer and logging both need.

### Mutually-exclusive transition enum

```rust
enum RuntimeTransition {
    Unchanged,
    ContextChanged,
    ActiveProfileChanged(Option<ProfileId>),
}
```

Rejected. This cannot represent the case where context and active profile
both change in the same observation. A structured struct is simpler and more
expressive.

### Debounce/stability window

Rejected for Sprint 2. Temporal debounce is deferred until testing proves
that naive deduplication causes observable flicker in connected clients.

## Consequences

- **Positive:** `RuntimeTransition` is a single source of truth for what
  changed in an observation cycle.
- **Positive:** `latest_context` retention on `Err` prevents spurious
  transitions from transient failures.
- **Positive:** Manual mode context recording ensures zero-delay automatic
  resolution on mode switch.
- **Negative:** `RuntimeTransition` carries both `previous_profile_id` and
  `active_profile_id`, duplicating data that already exists in
  `ProfileRuntime`. This is acceptable — the transition is a value type
  capturing a point-in-time snapshot.
- **Neutral:** Struct fields are more verbose than an enum match. This is
  acceptable for the increased expressiveness.
