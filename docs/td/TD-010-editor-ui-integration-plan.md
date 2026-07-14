# TD-010: Desktop Studio Editor UI Integration Plan

**Priority:** High
**Target:** EP-004 Sprint 4 (pre-implementation plan)

## Status

- **Phase:** EP-004 Sprint 4A — planning only. Implementation not started.
- **Prerequisite:** EP-004 Sprint 3B reducer foundation is certified and FROZEN.
- This document describes the minimal Editor UI that integrates egui with the
  existing `AppState::dispatch(Command)` pipeline.
- After this document passes independent review, Sprint 4B (implementation)
  may begin.

## Current GUI audit

### DesktopApp-owned UI state (`gui.rs`)

```rust
pub struct DesktopApp {
    pub state: SharedState,        // Arc<Mutex<AppState>>
    store: Arc<dyn DocumentStore>,  // persistence handle
    theme: Theme,                   // Dark / Light
    rename_device_id: String,       // orphan, unused in rendering
    rename_buffer: String,          // orphan, unused in rendering
}
```

**Orphan fields**: `rename_device_id` and `rename_buffer` were introduced for
an incomplete device rename feature and have no corresponding UI in the current
views. They are a warning sign of DesktopApp accumulating stale state.

### Existing tabs (`state.rs`)

| Tab | Handler | Purpose |
|-----|---------|---------|
| Dashboard | `show_dashboard` | Server status, recent actions |
| Devices | `show_devices` | Trusted device list, forget |
| Pairing | `show_pairing` | QR code + OTP display |
| Settings | `show_settings` | Auto-start, theme, exit |
| About | `show_about` | Version, platform info |

No Editor tab exists. `AppState::dispatch()` is wired but not called from any
UI path.

### Current mutation flow

- **Device mutations**: Direct `AppState` methods (`forget_device`,
  `rename_device`, `touch_device`, `add_device`). No Command dispatch.
  The caller persists via `store.save(&document)` after mutation.
- **Profile/Page/Button mutations**: None exist in the UI yet. All mutations
  pass through `command::apply()` but nothing constructs or dispatches
  `Command` values from the GUI.

### Monolith risks

1. Adding editor views inline in `gui.rs` would push it past ~550 lines.
2. No reusable egui helper widgets exist — every view builds inline frames.
3. The orphan `rename_*` fields show DesktopApp already accumulates stale
   ephemeral state with no cleanup discipline.

### Reusable egui helpers

None. Every view builds its own `Frame::group(...)` inline.

## Version metadata finding

| Source | Version |
|--------|---------|
| `Cargo.toml` | `0.2.0` |
| `ADR-007` | `v0.2+` (Release: v0.2) |
| `ADR-008` | `v0.3+` (Release: v0.3) |

**Contradiction**: ADR-008 claims `v0.3+` while the crate is `0.2.0`. ADR-007
is internally consistent with the crate.

**Resolution**: This contradiction is documented and deferred. No version
metadata has been changed. The resolution will be decided by project
governance when Sprint 4 implementation begins (bump the crate, correct
ADR-008, or both).

## ASCII Editor UI simulation

```
┌─────────────────────────────────────────────────────────────────┐
│  AutoMatDeck Desktop Studio                    ● Running        │
├──────────┬──────────────────────────────────────────────────────┤
│ Tab nav  │  Editor workspace                                     │
│          │                                                      │
│ 📊 Dash  │  ┌───────── Profile selector ──────────────────┐    │
│ 🖊 Editor │  │ [◄]  Gaming  │  Work  │  [Home]  │  [+]     │    │
│ 📱 Devices│  └──────────────────────────────────────────────┘    │
│ 🔗 Pairing│                                                      │
│ ⚙ Settings│  ┌── Page tabs ──────────────────────────────┐      │
│ ℹ About   │  │ [Controls]  [Apps]  [+]                   │      │
│          │  └────────────────────────────────────────────┘      │
│          │                                                      │
│          │  ┌── Button grid (4×4 with label text) ────────┐    │
│          │  │ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐       │    │
│          │  │ │Launch│ │ Calc │ │ Lock │ │  +   │       │    │
│          │  │ │Note..│ │      │ │      │ │      │       │    │
│          │  │ └──────┘ └──────┘ └──────┘ └──────┘       │    │
│          │  │ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐       │    │
│          │  │ │  +   │ │      │ │      │ │      │       │    │
│          │  │ │      │ │      │ │      │ │      │       │    │
│          │  │ └──────┘ └──────┘ └──────┘ └──────┘       │    │
│          │  └────────────────────────────────────────────┘    │
│          │                                                      │
│          │  ┌── Properties / Editor ───────────────────────┐   │
│          │  │ Label:  [Launch Notepad_______________]      │   │
│          │  │ Action: [launch ▼]                           │   │
│          │  │ Payload: {"app": "notepad.exe"}             │   │
│          │  │                                              │   │
│          │  │ [Save]  [Remove]  [Move →]                  │   │
│          │  └──────────────────────────────────────────────┘   │
│          │                                                      │
│          │  ┌── Error bar ───────────────────────────────┐    │
│          │  │ ⚠ Cannot delete last profile              │    │
│          │  └──────────────────────────────────────────────┘    │
├──────────┴──────────────────────────────────────────────────────┤
│  v0.2.0  │  3 trusted devices                                  │
└─────────────────────────────────────────────────────────────────┘
```

## UI state ownership table

All persistent domain state lives in `Document` (accessed through `AppState`).
All ephemeral editor state lives in `EditorUi` on `DesktopApp`.

### ID-based selection (corrected)

Use stable identity, not positional indices, for editor selections:

```rust
// EditorUi state (ephemeral, in DesktopApp)
struct EditorUi {
    // Selection — uses stable IDs, not indices
    selected_profile_id: Option<ProfileId>,
    selected_page_id: Option<PageId>,
    selected_button_id: Option<ButtonId>,

    // Draft buffers
    create_profile_name: String,
    edit_profile_name: String,
    create_page_name: String,
    edit_page_name: String,
    edit_button_label: String,
    edit_button_action: String,
    edit_button_payload: String,

    // Modal / confirmation flags
    show_create_profile: bool,
    show_rename_profile: bool,
    show_delete_profile_confirm: bool,
    show_rename_page: bool,
    show_delete_page_confirm: bool,
    target_move_page_id: Option<PageId>,  // for explicit MoveButton

    // Feedback
    last_command_error: Option<CommandError>,
}
```

**Rule**: Indices are temporary render-loop positions (e.g., `enumerate()`
inside an iterator). They must never be stored as long-lived editor selection
state. Use `Option<ProfileId>`, `Option<PageId>`, `Option<ButtonId>`.

### Ownership classification

| Field | Classification | Home |
|-------|---------------|------|
| `profiles` | persistent domain | `Document.profiles` |
| `pages` per profile | persistent domain | `Document.profiles[].pages` |
| `buttons` per page | persistent domain | `Document.profiles[].pages[].buttons` |
| `selected_profile_id` | ephemeral editor | `EditorUi` |
| `selected_page_id` | ephemeral editor | `EditorUi` |
| `selected_button_id` | ephemeral editor | `EditorUi` |
| `create_profile_name` | ephemeral editor | `EditorUi` |
| `edit_profile_name` | ephemeral editor | `EditorUi` |
| `create_page_name` | ephemeral editor | `EditorUi` |
| `edit_page_name` | ephemeral editor | `EditorUi` |
| `edit_button_label` | ephemeral editor | `EditorUi` |
| `edit_button_action` | ephemeral editor | `EditorUi` |
| `edit_button_payload` | ephemeral editor | `EditorUi` |
| `show_create_profile` | ephemeral editor | `EditorUi` |
| `show_rename_profile` | ephemeral editor | `EditorUi` |
| `show_delete_profile_confirm` | ephemeral editor | `EditorUi` |
| `show_rename_page` | ephemeral editor | `EditorUi` |
| `show_delete_page_confirm` | ephemeral editor | `EditorUi` |
| `target_move_page_id` | ephemeral editor | `EditorUi` |
| `last_command_error` | ephemeral editor | `EditorUi` |

## Command mapping table

| UI interaction | Command | Caller generates | Persist |
|---|---|---|---|
| Create Profile | `CreateProfile` | `ProfileId::new()`, `PageId::new()` | on success |
| Rename Profile | `RenameProfile` | — | on success |
| Delete Profile | `DeleteProfile` | — | on success |
| Select Profile | *(no command)* | — | no |
| Add Page | `AddPage` | `PageId::new()` | on success |
| Rename Page | `RenamePage` | — | on success |
| Delete Page | `DeletePage` | — | on success |
| Select Page | *(no command)* | — | no |
| Add Button | `AddButton` | `ButtonId::new()`, `Button` | on success |
| Edit Button label | `UpdateButton` | — | on success |
| Edit Button action | `UpdateButton` | — | on success |
| Remove Button | `RemoveButton` | — | on success |
| Move Button | `MoveButton` | — | on success |

Every persistent mutation maps to exactly one existing `Command` variant.

**Dispatch example** (see also orchestration rule below):

```rust
let cmd = Command::CreateProfile {
    profile_id: ProfileId::new(),
    initial_page_id: PageId::new(),
    name: self.create_profile_name.clone(),
};
match self.state.lock().unwrap().dispatch(&cmd) {
    Ok(()) => {
        self.persist(&self.state.lock().unwrap());
        self.editor.last_command_error = None;
    }
    Err(e) => {
        self.editor.last_command_error = Some(e);
    }
}
```

## Dispatch / persistence orchestration rule

Editor code must use a single small integration helper that:

1. Locks `AppState` (acquires `Mutex` guard).
2. Calls `dispatch(&cmd)`.
3. On success, persists via the existing `AppState::persist(store)` API
   (which calls `DocumentStore::save`).
4. Releases the state guard.
5. Returns `Result<(), CommandError>` to `EditorUi` for error display.

The helper is orchestration only. It must not:
- Mutate `Document` directly.
- Duplicate reducer logic.
- Construct IDs for commands (that remains the editor's responsibility).

**Actual API signatures** (from checked-in code):

```rust
// state.rs
pub fn dispatch(&mut self, cmd: &Command) -> Result<(), CommandError>
pub fn persist(&self, store: &dyn DocumentStore)

// repository.rs
pub trait DocumentStore: Send + Sync {
    fn load(&self) -> Document;
    fn save(&self, document: &Document);
    fn data_dir(&self) -> &Path;
}

// gui.rs — existing pattern
fn persist(&self, state: &AppState) {
    state.persist(&*self.store);
}
```

## gui.rs vs editor.rs decision

**Decision**: Extract `editor.rs` now.

**Rationale**: The Editor is already a distinct UI/state boundary.
- `gui.rs` handles application shell (title bar, tab nav, status bar, view routing).
- `editor.rs` owns `EditorUi` struct, all ephemeral editor state, and all
  editor rendering/interaction methods.

`gui.rs` is expected to grow from ~290 to ~350 lines (adding `Tab::Editor`
routing). `editor.rs` will begin at ~250–350 lines. This is not premature
abstraction — the current orphan field pattern and 5-tab structure already
show strain.

## Sprint implementation slices

Each slice keeps the app runnable. A `Tab::Editor` arm exists from 4B onward;
unimplemented features show a placeholder label.

### 4B — Editor shell + ID-based Profile/Page navigation

- Add `Tab::Editor` variant.
- Create `editor.rs` with `EditorUi` struct and `show_editor(ui, state, store)`.
- Render profile selector (list or tab bar using `selected_profile_id`).
- Render page tabs for selected profile using `selected_page_id`.
- Empty button area placeholder.
- No mutations yet.

### 4C — Profile/Page mutations

- Create Profile (form → `CreateProfile` → dispatch → persist).
- Rename Profile (inline or dialog → `RenameProfile`).
- Delete Profile (confirm → `DeleteProfile`, error on last).
- Add Page (form → `AddPage` → dispatch → persist).
- Rename Page (inline or dialog → `RenamePage`).
- Delete Page (confirm → `DeletePage`, error on last).
- Error bar rendering and auto-clear.

### 4D — Button grid + AddButton

- Render buttons for selected page in a 4×N grid.
- Empty slot (+) at end of grid.
- Click (+) → `AddButton` with `ButtonId::new()` and a default label/action.
- Grid updates from Document after dispatch.

### 4E — Button properties + Update/Remove

- Properties panel for selected button.
- Label text field, Action dropdown, Payload text area.
- Save button → `UpdateButton` → dispatch → persist.
- Remove button → `RemoveButton` → dispatch → persist.
- Error handling for missing button (race).

### 4F — Explicit target-page MoveButton + integration hardening

- Move Button: target-page selector (dropdown of all pages in current profile)
  + explicit "Move" action button.
- Moves dispatch `MoveButton` with explicit `to_page`.
- **No drag-and-drop** — drag-and-drop is deferred.
- Full CRUD smoke test: create profile → add pages → add buttons → edit →
  move → delete. Verify persistence survives restart.

## Risks and mitigations

| Risk | Mitigation |
|------|------------|
| Ephemeral state leaks into Document | Enforce ownership table in code review. No `&mut Document` mutations outside `dispatch()`. |
| Accidental direct mutation bypassing Commands | Hard rule: profile/page/button mutations route exclusively through `state.dispatch()`. |
| egui borrow issues around Mutex | `update()` already uses the extraction pattern (lock, read, drop before mutation). Editor follows same pattern. |
| No visible error feedback | `last_command_error: Option<CommandError>` on `EditorUi`; rendered as a colored frame that auto-clears on next successful dispatch. |
| gui.rs monolith even after extraction | Acceptable at ~350 lines. Further tab extraction (e.g., `tab_dashboard.rs`) is future work. |

## Expected implementation files

| File | Change | Notes |
|------|--------|-------|
| `apps/desktop/src/gui.rs` | Add `Tab::Editor` arm and `show_editor` wiring | Shell routing only |
| `apps/desktop/src/state.rs` | Add `Tab::Editor` variant to enum | No logic changes |
| `apps/desktop/src/editor.rs` | New file | `EditorUi` struct + all editor views |
| `apps/desktop/src/main.rs` | Add `mod editor;` | Module declaration |

**Frozen (no changes)**: `command.rs`, `model.rs`, `actions.rs`,
`repository.rs`, `Cargo.toml`, ADR-007, ADR-008.

## Related

- ADR-007: Immutable IDs (ID-based selection rationale)
- ADR-008: Command/reducer pattern (mutation routing architecture)
- TD-009: Command/reducer foundation (frozen Sprint 3B)
