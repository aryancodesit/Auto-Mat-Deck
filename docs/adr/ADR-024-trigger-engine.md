# ADR-024: Trigger Engine

**Status:** Accepted
**Date:** 2026-07-17
**Supersedes:** —
**Superseded By:** —

## Context

v0.6 established a complete execution pipeline: Android sends an opaque
button_id, Desktop resolves it to an ExecutionTarget (Action or Workflow),
and executes it. This works for user-initiated invocations but cannot
express autonomous automation — "when Chrome launches, lock the
workstation" or "at 9 AM, open my IDE."

Users need event-driven automation: triggers that observe context
changes, fire on schedule, or respond to manual invocations, and
dispatch to the existing execution pipeline without modifying it.

## Decision

### 1. Triggers are orchestration-layer objects, not execution-layer objects

A trigger is a first-class domain object owned by the Desktop. It
observes events and dispatches to ExecutionTarget — it never executes
directly. This preserves the v0.6 execution layer as a frozen,
testable boundary.

```
Context Observer ──┐
Timer Thread ──────┤
Android Manual ────┘
        │
        ▼
  Trigger Engine
  (evaluate + match)
        │
        ▼
  Trigger Dispatcher
  (sync → async bridge)
        │
        ▼
  ExecutionTarget ──── frozen v0.6
        │
        ▼
  execute_target()
```

### 2. TriggerType is an enum, not a trait

```rust
pub enum TriggerType {
    DesktopStartup,
    ProcessLaunch { process_name: String },
    Time { schedule: String },
    Manual,
}
```

A trait-based plugin system would be more extensible but introduces
dynamic dispatch, registration complexity, and makes structural
validation harder. An enum keeps triggers as simple data objects
that are easy to serialize, validate, and test. New trigger types
add a variant, not a new trait implementation.

### 3. Trigger.workflow_id references WorkflowId, not ExecutionTarget

```rust
pub struct Trigger {
    pub id: TriggerId,
    pub name: String,
    pub trigger_type: TriggerType,
    pub workflow_id: WorkflowId,  // not ExecutionTarget
    pub enabled: bool,
}
```

Changing to `target: ExecutionTarget` would generalize dispatch but
introduces migration complexity for a schema that is persisted to
disk. Keeping `workflow_id` means zero migration, zero protocol
breakage, and simpler persistence. The tradeoff is acceptable for
v0.7 and can be revisited if action-targeted triggers are needed.

### 4. Time triggers use UTC SystemTime, not local timezone

```rust
pub fn evaluate_time_triggers(triggers: &[Trigger], minute: u32, hour: u32)
```

SystemTime is deterministic, testable, and has no timezone
dependencies. UTC semantics are explicit and documented. Local-time
alignment would require a timezone database or OS local-time API,
both of which introduce platform dependency and testing complexity.
UTC is the correct default for a deterministic scheduler.

### 5. Schedule format is "minute hour" with `*` wildcard

```json
"0 9"       // every day at 09:00
"*/15 *"    // every 15 minutes
"* *"       // every minute
```

Cron-like but simplified. No day-of-week, no month, no complex
expressions. The schedule is parsed by `schedule_matches()` which
compares each field independently. Wildcard `*` matches any value.
Numeric fields must match exactly. This covers the common cases
without introducing a cron parser.

### 6. Triggers live in Document

```
Document
  ├── profiles: Vec<Profile>
  ├── context_rules: Vec<ContextRule>
  ├── workflows: Vec<Workflow>
  └── triggers: Vec<Trigger>
```

Follows ADR-001 (Desktop owns configuration) and ADR-008
(Command/Reducer pattern). Triggers flow through the same mutation
pipeline, persistence layer, and reconciliation logic as profiles,
context rules, and workflows.

### 7. Validation is split into Structural and Execution phases

**Structural Validation** operates only on serialized trigger data.
No I/O, no runtime state, no registry lookups.

- Trigger ID is non-empty
- Trigger name is non-empty
- Workflow ID is non-empty
- Time triggers have non-empty schedule
- No duplicate trigger IDs
- TriggerVersion is supported

**Execution Validation** consults runtime services during invocation.

- Workflow exists in Document at invocation time
- Workflow is enabled
- Referenced workflow is executable

### 8. Trigger Dispatcher bridges sync and async

The observer thread and timer thread are synchronous (matching the
existing observer architecture). The execution pipeline is async
(tokio). `TriggerDispatcher` holds a tokio runtime handle and
spawns async execution from the synchronous context.

```rust
pub struct TriggerDispatcher {
    runtime: tokio::runtime::Handle,
}
```

This avoids converting the entire observer architecture to async
while still leveraging the existing async execution pipeline.

### 9. Android receives trigger state, cannot create triggers

The Desktop pushes a `trigger_state` message containing the trigger
list. Android renders triggers with enabled/disabled status and
provides fire buttons for manual triggers. Android cannot create,
edit, or delete triggers — that remains Desktop-authoritative.

This follows ADR-001 (Desktop owns configuration) and keeps the
Android app as a thin client.

### 10. Observer thread evaluates triggers on context change

The existing observer thread (which detects foreground process
changes) also evaluates triggers. When a context change occurs:

1. Observer detects new foreground process
2. `evaluate_context_change()` maps the snapshot to matching triggers
3. `TriggerDispatcher` dispatches each result to `execute_target()`

This reuses the existing observer infrastructure without adding
a separate event bus.

## Consequences

### Positive

- Preserves every v0.6 architectural boundary
- No execution-layer changes required
- Triggers are simple data objects (easy to serialize, validate, test)
- Time triggers are deterministic and testable
- Android trigger UI is minimal (display + fire)
- Document schema extends cleanly (new `triggers` field)
- Structural validation is independent of execution infrastructure
- TriggerType enum is closed (easy to reason about all variants)

### Negative

- Triggers cannot target Actions directly (workflow_id only)
- Time triggers are UTC-only (no local timezone)
- Schedule format is limited (no day-of-week, no month)
- Observer thread polls on a fixed interval (not event-driven)
- No trigger history or audit log
- No compound trigger conditions (AND/OR)

### Risks

- Timer thread precision (60s poll) may miss short-lived time windows.
  Mitigated by the fact that time triggers represent "at this minute"
  not "at this exact second."
- If trigger count grows large, observer thread evaluation could
  become a bottleneck. Mitigated by linear scan being sufficient for
  expected trigger counts (< 100).

## Compliance

- Trigger definitions MUST be Desktop-authoritative (ADR-001)
- Trigger mutations MUST use the Command/Reducer pattern (ADR-008)
- Triggers MUST dispatch through ExecutionTarget, never execute directly
- Structural validation MUST NOT depend on runtime registries
- Android MUST NOT receive trigger definitions or creation capabilities
- Time triggers MUST use UTC semantics (documented)
- TriggerVersion MUST be present in all stored triggers
