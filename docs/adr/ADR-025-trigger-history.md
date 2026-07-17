# ADR-025: Trigger History

**Status:** Accepted
**Date:** 2026-07-17
**Supersedes:** —
**Superseded By:** —

## Context

v0.7 introduced a trigger engine that dispatches to ExecutionTarget, but
provided no visibility into what triggers fired, when, or whether they
succeeded. Users have no execution log — they cannot tell if a time
trigger ran at 9 AM or why a process-launch trigger failed.

The execution layer (v0.6) is frozen. Trigger dispatch is the natural
place to record history: it is the single sync→async bridge through
which every trigger passes.

## Decision

### 1. TriggerHistory is a bounded ring buffer

```rust
pub struct TriggerHistory {
    records: VecDeque<TriggerExecutionRecord>,
    max_size: usize, // default 100
}
```

VecDeque with front-eviction at capacity. O(1) push, bounded memory,
no allocation churn. The cap is configurable but defaults to 100 —
enough for a day of active automation, small enough to serialize and
send over WebSocket without buffering concern.

### 2. TriggerDispatcher records every dispatch

`TriggerDispatcher` holds `Arc<Mutex<TriggerHistory>>` and records
each dispatch with trigger_id, workflow_id, status, timestamp, and
duration_ms before handing off to `execute_target()`. Recording happens
before async execution so the record exists even if execution panics.

```rust
shared_history.lock().record(
    trigger_id, workflow_id,
    TriggerExecutionStatus::Success,  // updated after execution
    now_unix(), duration_ms,
);
```

Status is initially `Success` — the dispatcher sets it based on
`execute_target()` outcome. Failed dispatches are recorded with
`TriggerExecutionStatus::Failed { reason }`.

### 3. Watch channel pushes live updates to connected Android clients

```rust
let (history_tx, history_rx) = watch::channel::<Option<String>>(None);
```

After recording, `TriggerDispatcher` publishes serialized history via
`watch::Sender<Option<String>>`. The agent threads the `Receiver` and:

- Sends a retained snapshot on trusted connect (all 3 paths: initial
  identify→trusted, otp_pair_accepted, tray approval)
- Forwards live updates in the `select!` loop

Wire format:
```json
{
  "type": "trigger_history",
  "schema_version": 1,
  "records": [
    {
      "trigger_id": "t1",
      "workflow_id": "wf1",
      "status": "Success",
      "timestamp": 1689600000,
      "duration_ms": 150
    }
  ]
}
```

Records are newest-first (reversed from VecDeque order). The receiver
gets the latest snapshot — if it misses an update, it still has the
most recent state (watch semantics).

### 4. Persistence via JSON file

```rust
impl TriggerHistory {
    pub fn save_to_file(&self, path: &Path) { ... }
    pub fn load_from_file(path: &Path, max_size: usize) -> Self { ... }
}
```

- **Save:** `serde_json::to_string` → `fs::write` to `trigger_history.json`
  in the data directory. Called on graceful shutdown.
- **Load:** On startup, reads and deserializes. Missing file → empty
  history. Malformed file → empty history with warning. Records clamped
  to max_size on load (in case file has more than current cap).
- Format: flat JSON array of `TriggerExecutionRecord` objects.

### 5. Android renders an execution log

Android receives `trigger_history` messages and renders an execution log
section below the trigger list. Each record shows:
- Status icon: ✅ Success, ❌ Failed, ⏭ Rejected
- Trigger name (resolved from trigger_id)
- Duration in ms
- Failure reason if applicable

The log updates live as new `trigger_history` messages arrive.

### 6. Serde default enum serialization

`TriggerExecutionStatus` uses `#[serde(rename_all = "PascalCase")]`:
- `Success` → `"Success"`
- `Failed { reason }` → `{"Failed":{"reason":"..."}}`

The Kotlin parser matches on `"Success"`, `"Failed"`, `"Rejected"` —
no raw enum variant handling needed.

## Consequences

### Positive

- Users can see what triggers fired and whether they succeeded
- Bounded memory regardless of automation volume
- Watch channel gives live updates without polling
- Retained snapshot means newly connected clients see history
- Persistence across restarts (graceful shutdown saves, startup loads)
- Simple JSON format — human-readable, debuggable
- Android execution log provides immediate visual feedback

### Negative

- History is in-memory + single JSON file (not a database)
- No history search/filter (linear scan only)
- No history export or persistence rotation
- Watch channel sends full history on each update (not incremental)
- No per-trigger history isolation

### Risks

- watch channel sends serialized history on every dispatch. At default
  cap (100 records) this is ~2-3 KB — negligible. If cap is increased
  significantly, consider incremental updates.
- `Mutex<TriggerHistory>` held during record insertion. The critical
  section is O(1) push + optional O(1) pop — no contention concern.

## Compliance

- TriggerHistory MUST record every dispatch attempt (pre-execution)
- History MUST be bounded (configurable max_size, default 100)
- Watch channel MUST publish after recording (not after execution)
- Agent MUST send retained snapshot on trusted connect
- Persistence MUST handle missing/malformed files gracefully
- Android MUST render execution log from trigger_history messages
- Serde serialization MUST use PascalCase for enum variants
