# ADR-010: Desktop Runtime Ownership

**Status:** Accepted
**Date:** 2026-07-13

## Context

v0.3 introduces `ProfileRuntime` — the active runtime Profile state,
`SelectionMode`, and latest `ContextSnapshot`. This is not persisted state.

The question is: where does `ProfileRuntime` live relative to `AppState`
(persisted Document + GUI flags)?

The current shared-state architecture is:

```rust
// state.rs — existing
pub type SharedState = Arc<Mutex<AppState>>;

pub struct AppState {
    pub document: Document,
    pub server_running: bool,
    pub selected_tab: Tab,
}
```

Three threads share a single `Arc<Mutex<AppState>>`:

| Thread | Access |
|--------|--------|
| Main (eframe/GUI) | Reads document + server_running + selected_tab every frame |
| Tokio (agent) | Reads document for trust checks, writes document on pairing |
| Tray pump | Sets `server_running = true` at startup |

## Decision

**Create `DesktopRuntime` wrapping `AppState` + `ProfileRuntime` under one lock.**

```rust
pub struct DesktopRuntime {
    pub app: AppState,
    pub runtime: ProfileRuntime,
}

pub type SharedRuntime = Arc<Mutex<DesktopRuntime>>;
```

This replaces `SharedState` (the type alias is removed; `SharedRuntime`
becomes the new shared type).

## Alternatives Considered

### A. ProfileRuntime inside AppState

```
AppState {
    document,
    server_running,
    selected_tab,
    runtime,           // ProfileRuntime here
}
```

**Rejected.** Mixes persisted state, GUI state, and high-frequency runtime
state in the same struct. This is how god-state architectures begin. v0.3
establishes the separation before ProfileRuntime becomes complex.

### B. Two independent mutexes

```
SharedState {
    app: Mutex<AppState>,
    runtime: Mutex<ProfileRuntime>,
}
```

**Rejected.** No coherence guarantee — reading Document to validate a
ProfileId while another thread mutates runtime requires external
synchronization. Every lock site changes. More complex than single-lock.

## Coherence

A single `Arc<Mutex<DesktopRuntime>>` guarantees:

- Both `app.document` and `runtime.active_profile_id` are read/written
  atomically from any thread.
- Document mutations always see a consistent ProfileRuntime.
- ProfileRuntime validation against Document never sees a stale Document.

## DesktopRuntime guard

`DesktopRuntime` initially contains exactly two fields:

- `app: AppState`
- `runtime: ProfileRuntime`

A third field requires explicit architecture review via an ADR amendment.
This prevents DesktopRuntime from silently becoming the universal god-state
container.

## Migration from SharedState

Mechanical rename across 3 threads:

| File | Change |
|------|--------|
| `state.rs` | Remove `SharedState` alias. Add `DesktopRuntime`, `SharedRuntime`, `new_shared()` returning `SharedRuntime`. |
| `main.rs` | `app_state` → `runtime`. Type changes. |
| `gui.rs` | `state: SharedState` → `state: SharedRuntime`. Accessor: `.lock().unwrap().app` for GUI reads. |
| `agent.rs` | Parameter type changes. Accessor: `.lock().unwrap().app` for Document reads. |
| `tray.rs` | Parameter type changes. |

No behavioral change. No new lock contention. No new lock ordering.

## Runtime mutation is not Command/Reducer-bound

Foreground changes, active Profile changes, and SelectionMode changes do NOT
become `Command` variants. The `Command`/`AppState::dispatch` boundary remains
**Document mutation only**.

However, runtime must validate against Document. Because both live under one
`Arc<Mutex<DesktopRuntime>>`, a code path can:

1. Lock the mutex.
2. Mutate `self.app.document` via `dispatch()`.
3. Mutate `self.runtime` in the same scope.
4. Unlock.

The lock guarantees atomicity. The reducer (`command::apply`) itself never
touches runtime state. The caller orchestrates both mutations within the same
lock scope.

## Profile deletion lifecycle

When `Command::DeleteProfile` is dispatched:

1. `command::apply(&doc, &cmd)` returns a new `Document` without the Profile.
   ContextRules targeting the deleted Profile are also cascade-removed in the
   same reducer call.
2. The caller (e.g. `dispatch_editor` in `gui.rs`) holds the
   `DesktopRuntime` lock.
3. `self.app.document = new_doc` — installs the new Document.
4. `self.runtime.reconcile(&self.app.document)` — validates
   `active_profile_id` against the new Document. If stale, the resolver
   re-runs to determine the new active Profile.
5. Lock released.

This keeps the reducer pure and the runtime reconciliation as a caller
responsibility, both under one coherent lock boundary.
