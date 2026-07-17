# ADR-022: Opaque Control Invocation

**Status:** Accepted
**Date:** 2026-07-15

## Context

ADR-021 established the Desktop-authoritative control surface projection:
Mobile clients receive `control_surface_state` frames describing the active
profile's pages and buttons as opaque items.

v0.5 Sprint 2 proved complete CSS consumption on Android: receive, decode,
retain semantic state, and render native control Button views.

v0.5 Sprint 3 extends the protocol in the reverse direction: Mobile clients
may request invocation of a projected control by its opaque identity, with
the Desktop solely responsible for validating and (in a later sprint) executing
the request.

## Decision

### 1. Android transmits intent, not execution metadata

The Mobile client sends only the opaque `button_id` that the Desktop
projected. The Desktop does not delegate any action knowledge to the Mobile
client:

```json
{
  "type": "control_invoke",
  "schema_version": 1,
  "button_id": "opaque-button-id"
}
```

The Mobile client MUST NOT include `action`, `action_type`, `payload`,
`command`, `path`, `args`, `profile_id`, `page_id`, or any Desktop-owned
context.

### 2. Desktop is the sole authority for active profile resolution

button_id is a **per-page-unique** opaque string (see ADR-007). The Desktop
alone knows the current active profile and resolves the button_id against its
authoritative `Document.profiles`. The Mobile client never asserts which
profile is active.

### 3. Validation precedes execution

Every incoming `control_invoke` request passes through a pure validation
function before any execution is considered.

```rust
fn validate_button<'a>(
    active_profile_id: Option<&ProfileId>,
    profiles: &'a [Profile],
    button_id: &str,
) -> Result<&'a Button, RejectionReason>
```

Validation queries the current active Profile only. Historical presence,
cached state, or Mobile assertions must not influence validation.

### 4. Three rejection reasons

| Reason | Meaning | Source |
|---|---|---|
| `unknown_button` | 0 buttons in the current active profile matched the requested button_id | Stale projection |
| `no_active_profile` | No profile is currently active | Legitimate runtime state |
| `ambiguous_button` | >1 buttons in the current active profile matched the requested button_id | Configuration integrity failure |

`ambiguous_button` is a configuration integrity failure, not a transport or
networking failure. Future editor tooling (import validation, diagnostics)
should flag duplicate button_ids as a configuration quality issue. The
protocol rejection reason acts as defense-in-depth.

### 5. Execution layers consume validated Button references

Sprint 3 stops after validation, returning:

```json
{
  "type": "control_invoke_result",
  "schema_version": 1,
  "button_id": "opaque-button-id",
  "accepted": false,
  "reason": "unknown_button"
}
```

Sprint 4 adds execution semantics. When `accepted=true`, the Desktop
resolves the `Button.action` into an `ActionReference` and dispatches
execution via the `ActionRegistry`. The response extends with two fields:

```json
{
  "type": "control_invoke_result",
  "schema_version": 1,
  "button_id": "opaque-button-id",
  "accepted": true,
  "executed": true
}
```

On execution failure:

```json
{
  "type": "control_invoke_result",
  "schema_version": 1,
  "button_id": "opaque-button-id",
  "accepted": true,
  "executed": false,
  "execution_error": "execution_timeout"
}
```

| Execution error | Meaning |
|---|---|
| `execution_failed` | Action returned an error |
| `action_not_found` | Button references an unknown action name |
| `execution_timeout` | Action exceeded 5s timeout |
| `execution_panicked` | Action panicked (caught via `catch_unwind`) |

`executed` and `execution_error` are absent when `accepted=false`.
`execution_error` is absent when `executed=true`.

The `ActionRegistry.execute()` is synchronous. Execution is wrapped in
`spawn_blocking` + `tokio::time::timeout(5s)` to avoid blocking the async
runtime and to enforce a hard timeout.

### 6. Validation is pure and transport-independent

`validate_button()` has no I/O, no logging, no metrics, no WebSocket access,
and no access to the action registry. It operates solely on:
- the active profile identity,
- the Document's profile list,
- the requested button_id.

This makes it testable without async, networking, or Android infrastructure.

### 7. ControlSurfaceUiState remains projection-only

Invocation results do not modify `ControlSurfaceUiState`. The Mobile client
displays the result diagnostically (transient table) without adding invocation
state to the projection model.

### 8. Ok(&Button) does not imply execution

Returning `Ok(&Button)` from `validate_button()` indicates only that the
requested button was uniquely and structurally resolved within the current
active profile. It does not imply authorization, execution, scheduling,
dispatch, or successful completion of any action. Sprint 3 proves the
transport and authority model; a later sprint adds execution semantics.

### 9. Unknown fields in protocol frames must be ignored

Receivers of `control_invoke` and `control_invoke_result` frames must ignore
unknown fields unless required by a future schema version bump. This ensures
protocol extensibility without a schema version increment for every optional
addition.

## Consequences

### Positive

- Clean separation of validation (Sprint 3) from execution (Sprint 4).
- `validate_button()` is reused verbatim — no changes needed for execution.
- Deterministic rejection rules with no hidden first-match-wins semantics.
- No Mobile awareness of Desktop-owned context (action names, payloads).
- `Ok(&Button)` is explicitly scoped to structural resolution, not execution.
- Unknown-field-extensibility ensures forward compatibility.
- Timeout and panic handling prevent hung actions from blocking the runtime.

### Negative

- `ambiguous_button` adds a third rejection code whose root cause is a
  configuration problem, not a runtime error. Requires educating Desktop
  tooling developers about this distinction.

### Risks

- If button_id uniqueness is eventually enforced profile-wide by the editor,
  `ambiguous_button` becomes unreachable in normal operation. The protocol
  rejection reason is still retained as defense-in-depth; removing it later
  would be a breaking protocol change.
- Sprint 3+4 implementers must consciously avoid calling `ACTIONS.execute()`,
  `shell_execute()`, or the action registry near the validation boundary.
- The 5s timeout is a hard limit; actions that legitimately take longer
  (e.g. large file operations) will be killed. This is acceptable for
  HID/GPIO actions; revisit if longer-running actions are added.

## Compliance

- Every `control_invoke` request MUST have its button_id validated against
  the Desktop's current active control surface.
- The validation layer MUST NOT execute actions, launch processes, or mutate
  the system.
- Execution MUST only occur when `accepted=true` and the validated Button
  has a valid `action` field.
- Execution MUST be wrapped in `spawn_blocking` + `timeout(5s)` to prevent
  blocking the async runtime.
- Panics in action execution MUST be caught via `catch_unwind` and reported
  as `execution_panicked`.
- The Mobile client MUST NOT add profile_id or page_id to the request frame.
- Validation MUST NOT depend on cached or historical projection state.
