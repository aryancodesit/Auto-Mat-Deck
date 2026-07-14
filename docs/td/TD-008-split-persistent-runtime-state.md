# TD-008: Split PersistentState from RuntimeState

**Priority:** Low
**Target:** Before automation / context features

## Description

Currently, `AppState` mixes two concerns:

- **Persistent state** — `Document` (devices, profiles, pages, buttons)
- **Runtime state** — `server_running`, `selected_tab`

As the project grows, runtime state will include connected-device
sessions, notifications, undo history, selection, and drag state. Mixing
these with persisted data creates unnecessary save cycles and makes
it harder to reason about what gets persisted.

## Recommendation

```rust
struct AppState {
    persistent: PersistentState,   // → Document on disk
    runtime: RuntimeState,         // never persisted
}

struct PersistentState {
    document: Document,
}

struct RuntimeState {
    server_running: bool,
    selected_tab: Tab,
    connected_devices: Vec<Session>,
    undo_stack: Vec<Command>,
    // ...
}
```

## Do not implement yet

The current state is manageable. Split when automation or editor features
add enough runtime state to make the distinction meaningful.
