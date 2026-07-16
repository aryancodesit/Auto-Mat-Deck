# Sprint 4 Design Package: Control Invocation Execution

**Status:** Draft
**Date:** 2026-07-16
**Depends on:** ADR-022 (Sprint 3), Checkpoint 2 (Desktop transport), Checkpoint 3 (Android transport)

## 1. Execution Architecture

Sprint 3 proved the validation transport. Sprint 4 connects validation to execution.

```
Android
    │
    │  control_invoke { button_id }
    ▼
agent.rs (transport boundary)
    │
    │  clone (active_profile_id, profiles) from runtime lock
    │  release lock
    ▼
validate_button(active_pid, profiles, button_id)
    │
    │  Result<&Button, RejectionReason>
    ▼
┌─────────────────────────────────┐
│ Ok(button) → extract owned data │
│   action_name = button.action.action_name.clone()
│   payload     = button.action.payload.clone()
└─────────────┬───────────────────┘
              │
              ▼
    execute_action(action_name, payload)
    │
    │  tokio::time::timeout(5s, spawn_blocking(|| ACTIONS.execute(...)))
    ▼
    ExecutionOutcome { Success | Failed(msg) | Timeout }
              │
              ▼
    control_invoke_result { accepted, executed, execution_error? }
```

### Invariants preserved

- **agent.rs remains a transport boundary.** It validates, extracts owned data, then delegates execution. It never inspects action internals.
- **validate_button() remains pure.** No I/O, no transport, no execution knowledge.
- **ActionRegistry.execute() remains synchronous.** Wrapped in `spawn_blocking` at the call site, not inside the registry.
- **Runtime lock is never held during execution.** Profiles are cloned out before validation; action data is cloned out before execution.

## 2. Sequence Diagram

```
Android              Desktop agent.rs         validate_button    execute_action    ActionRegistry
  │                       │                          │                 │                │
  │── control_invoke ────▶│                          │                 │                │
  │                       │── clone runtime state ──▶│                 │                │
  │                       │   (release lock)          │                 │                │
  │                       │── validate ──────────────▶│                 │                │
  │                       │   Result<&Button>         │                 │                │
  │                       │◀──────────────────────────│                 │                │
  │                       │                          │                 │                │
  │                       │── extract action_name ────────────────────▶│                │
  │                       │   + payload (owned)       │                 │                │
  │                       │                          │                 │── execute ────▶│
  │                       │                          │                 │   Result<V, E>  │
  │                       │                          │                 │◀───────────────│
  │                       │◀── ExecutionOutcome ──────────────────────│                 │
  │                       │                          │                 │                │
  │◀─ control_invoke_result ─│                         │                 │                │
  │   (accepted + executed)  │                         │                 │                │
```

## 3. Response Schema Extension

Sprint 4 extends `control_invoke_result` with execution fields. Schema version remains 1 (additive, backward-compatible per ADR-022 §9).

### Validation rejected (unchanged from Sprint 3)

```json
{
  "type": "control_invoke_result",
  "schema_version": 1,
  "button_id": "opaque-id",
  "accepted": false,
  "reason": "unknown_button"
}
```

### Validation accepted, execution succeeded

```json
{
  "type": "control_invoke_result",
  "schema_version": 1,
  "button_id": "opaque-id",
  "accepted": true,
  "executed": true
}
```

### Validation accepted, execution failed

```json
{
  "type": "control_invoke_result",
  "schema_version": 1,
  "button_id": "opaque-id",
  "accepted": true,
  "executed": false,
  "execution_error": "execution_failed"
}
```

### Field semantics

| Field | Type | Present when | Meaning |
|-------|------|-------------|---------|
| `accepted` | bool | always | Validation gate: did the button resolve? |
| `executed` | bool | `accepted == true` | Did execution complete? |
| `execution_error` | string | `executed == false` | Categorized failure code |
| `reason` | string | `accepted == false` | Validation rejection code (Sprint 3) |

### Backward compatibility

Sprint 3 clients ignore unknown fields (ADR-022 §9). The new `executed` and `execution_error` fields are additive. No schema version bump required.

## 4. Failure Taxonomy

### Validation failures (Sprint 3, unchanged)

| Code | Meaning | Source |
|------|---------|--------|
| `no_active_profile` | No profile is currently active | Runtime state |
| `unknown_button` | 0 buttons matched in active profile | Stale projection |
| `ambiguous_button` | >1 buttons matched in active profile | Configuration integrity |

### Execution failures (Sprint 4, new)

| Code | Meaning | Source |
|------|---------|--------|
| `execution_failed` | Action ran but returned an error | Action impl |
| `action_not_found` | Action name not in registry | Configuration integrity |
| `execution_timeout` | Action did not complete within 5s | Timeout guard |
| `execution_panicked` | Action task panicked | Spawn failure |

### Mapping from ActionError

```rust
match ACTIONS.execute(&action_name, &payload) {
    Ok(_data)    → ExecutionOutcome::Success
    Err(e)       → ExecutionOutcome::Failed(classify_action_error(&e))
}

fn classify_action_error(e: &ActionError) -> ExecutionOutcome {
    if e.message.starts_with("Unknown action:") {
        ExecutionOutcome::ActionNotFound
    } else {
        ExecutionOutcome::Failed(e.message)
    }
}
```

### Distinction from validation

Validation failures mean "this request should never have been sent" (stale projection, no profile). Execution failures mean "the request was valid but the operation failed at runtime." These are different layers and must never be conflated into the same field.

## 5. Cancellation Model

**No cancellation.** All current actions are fire-and-forget OS operations:

| Action | Cancellable? | Reason |
|--------|-------------|--------|
| `launch` | No | ShellExecuteW returns immediately |
| `open_url` | No | ShellExecuteW returns immediately |
| `open_file` | No | ShellExecuteW returns immediately |
| `lock` | No | LockWorkStation returns immediately |
| `notify` | No | Toast creation returns immediately |

If the WebSocket disconnects during execution, the action still completes. The result is lost (connection gone), but the side effect is not undone. This is correct behavior — you cannot un-launch an app or un-lock a workstation.

If a future action needs cancellation (e.g., a long-running file copy), it should implement its own abort mechanism. The execution layer does not provide generic cancellation.

## 6. Timeout Behavior

### Per-execution timeout: 5 seconds

```
tokio::time::timeout(Duration::from_secs(5), spawn_blocking(|| execute()))
```

**Why 5 seconds:** Current actions complete in <500ms. 5s is a 10x safety margin that protects against OS hangs without being so generous that Android users wait indefinitely.

**On timeout:** Returns `ExecutionOutcome::Timeout`. The `control_invoke_result` carries `executed: false, execution_error: "execution_timeout"`. The action may still complete in the background — there is no way to abort a `spawn_blocking` task from outside.

**On panic:** `spawn_blocking` catches panics via `JoinHandle`. Returns `ExecutionOutcome::Panicked`. The action is not retried.

### No global rate limiting

Sprint 4 does not add rate limiting. Actions are triggered by explicit user taps on Android buttons. Accidental rapid taps are handled by the timeout guard. If abuse becomes a concern, add per-action cooldowns in a later sprint.

## 7. Telemetry / Events

All events are `log::info!` / `log::warn!` calls. No external telemetry system.

### Inbound

```
INFO  control_invoke from {peer}: button_id={id}
```

Logged on receipt, before validation.

### Validation

```
INFO  control_invoke_validated: button_id={id}, accepted={bool}, reason={reason?}
```

Logged after `validate_button()` returns.

### Execution

```
INFO  control_invoke_executed: button_id={id}, action={name}, executed={bool}, duration_ms={ms}
WARN  control_invoke_execution_failed: button_id={id}, action={name}, error={msg}, duration_ms={ms}
```

Logged after `execute_action()` returns. Duration is wall-clock time from `spawn_blocking` dispatch to result.

### Connection lifecycle (unchanged)

```
INFO  [CONNECT] Incoming TCP connection from {peer}
INFO  Connection closed: {peer} ({id})
```

## 8. Rollback Strategy

**No rollback.** This is a deliberate design decision, not a gap.

Rationale:
- All current actions (`launch`, `open_url`, `open_file`, `lock`, `notify`) are single-direction OS operations.
- There is no reliable inverse for `ShellExecuteW("open", "notepad.exe")` — the process may have spawned child processes, modified state, or been intercepted by another application.
- `LockWorkStation` has no undo.
- `Toast` is fire-and-forget.

If a future action requires undo (e.g., "undo last file operation"), it must implement its own rollback logic within the `Action` trait. The execution layer provides no generic rollback mechanism.

**Documented as a known limitation in ADR-022.**

## 9. Required ADR Updates

### ADR-022 modifications

| Section | Current | Sprint 4 |
|---------|---------|----------|
| §5 | "Sprint 3 stops after validation, returning..." | Extend: Sprint 4 resolves validated Button → ActionReference → ActionRegistry.execute() |
| §8 | "Ok(&Button) does not imply execution" | Amend: Sprint 4, Ok(&Button) implies execution. Sprint 3 behavior retained for reference. |
| Response schema | `accepted` + `reason` only | Add `executed` + `execution_error` fields |
| New §10 | — | Execution timeout (5s), panic handling, no cancellation |
| New §11 | — | Failure taxonomy: validation vs execution (never conflated) |
| Consequences §Neg | — | Add: actions are not rollback-able; timeout may leave orphaned background work |

### New file: ADR-023 (optional, if ADR-022 grows too large)

If the ADR-022 update exceeds ~200 lines, extract execution semantics into ADR-023: "Control Invocation Execution." ADR-022 would reference ADR-023 for execution details.

**Recommendation:** Keep it in ADR-022 for now. ADR-022 already owns the full request-response lifecycle. Split only if the document becomes unwieldy.

## 10. File Changes (Sprint 4, planned)

| File | Change |
|------|--------|
| `actions.rs` | Add `ExecutionOutcome` enum, `execute_button()` async wrapper |
| `agent.rs` | Extend `control_invoke` handler: extract action data, call `execute_button()`, build extended response |
| `ADR-022-control-invoke.md` | Update §5, §8, add §10, §11, extend response schema |
| `SpikeMessageDispatcher.kt` | Parse new `executed` / `execution_error` fields |
| `ControlInvokeRequest.kt` | No change (request unchanged) |
| Tests | Path G: execution tests (success, failure, timeout, panic) |

## 11. Test Plan

### Path G: Execution tests

| Test | Input | Expected |
|------|-------|----------|
| G1: accepted + executed | valid button_id, working action | `accepted: true, executed: true` |
| G2: accepted + execution_failed | valid button_id, action returns error | `accepted: true, executed: false, execution_error: "execution_failed"` |
| G3: accepted + action_not_found | valid button_id, action name not in registry | `accepted: true, executed: false, execution_error: "action_not_found"` |
| G4: accepted + execution_timeout | valid button_id, action hangs >5s | `accepted: true, executed: false, execution_error: "execution_timeout"` |
| G5: validation rejected (unchanged) | unknown button_id | `accepted: false, reason: "unknown_button"` |

### Existing test coverage preserved

- Path D: serialization (5 tests) — unchanged
- Path E: validate_button (8 tests) — unchanged
- Path F: dispatcher parsing (7 tests) — extended for new fields
- Desktop: 222 tests — execution tests added
