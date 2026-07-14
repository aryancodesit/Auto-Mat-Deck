# TD-009: Command/Reducer architecture for domain mutations

**Priority:** Medium
**Target:** EP-004 Sprint 3 (implemented)

## Status

**Foundation implemented.** See `src/command.rs`.

The following are implemented:
- `Command` enum (profiles, pages, buttons) — identity supplied by caller
- `apply(doc, cmd) -> Result<Document, CommandError>` reducer
- Invariant enforcement (last profile, last page, missing IDs, duplicate buttons)
- Correct error precedence (resolve entity before invariant check)
- Determinism: identical inputs produce identical outputs
- `AppState::dispatch(cmd)` integration
- 22 unit tests covering all operations, error paths, error precedence, determinism, and identity ownership

### Sprint 3A hardening (2026-07-12)

- **Identity moved out of reducer:** `CreateProfile` and `AddPage` commands
  now carry `profile_id` and `page_id` as caller-supplied fields. The reducer
  never calls `ProfileId::new()`, `PageId::new()`, `ButtonId::new()`, or
  `new_id()`.
- **Error precedence corrected:** `DeleteProfile` resolves the profile ID
  before checking `len() <= 1`. `DeletePage` resolves both profile and page
  IDs before checking page count. This means a missing ID on a single-profile
  document returns `ProfileNotFound`, not `CannotDeleteLastProfile`.

### Sprint 3B hardening (2026-07-12)

- **Complete identity ownership:** `CreateProfile` now requires
  `initial_page_id` as a caller-supplied field. The reducer no longer uses
  `PageId::from_string("default")` or any magic/constant ID string for
  persistent entity identity. The reducer constructs zero persistent entity
  IDs — all identity comes through `Command` fields.
- **ID scope documented in ADR-007:** `PageId` and `ButtonId` are scoped to
  their parent container, not globally unique within Document.

## Remaining (deferred)

- **Undo/redo:** Requires either inverse commands or snapshot stack. Not needed until editor has multi-step workflows.
- **Command serialization:** Would enable audit log / sync. Defer until there is a consumer.
- **Network synchronization:** Commands could be sent to connected mobile clients. Defer until protocol is ready.
- **Device command migration:** `forget_device`, `rename_device`, `add_device`, `touch_device` remain as `AppState` methods. They could move into `Command` if undo/redo is needed for device operations.

## Design

### Command enum

```rust
pub enum Command {
    CreateProfile { profile_id: ProfileId, initial_page_id: PageId, name: String },
    DeleteProfile { profile_id: ProfileId },
    RenameProfile { profile_id: ProfileId, new_name: String },
    AddPage { profile_id: ProfileId, page_id: PageId, name: String },
    DeletePage { profile_id: ProfileId, page_id: PageId },
    RenamePage { profile_id: ProfileId, page_id: PageId, new_name: String },
    AddButton { profile_id: ProfileId, page_id: PageId, button: Button },
    RemoveButton { profile_id: ProfileId, page_id: PageId, button_id: ButtonId },
    UpdateButton { profile_id: ProfileId, page_id: PageId, button: Button },
    MoveButton { profile_id: ProfileId, from_page: PageId, button_id: ButtonId, to_page: PageId },
}
```

Identity is always caller-supplied. The reducer never generates IDs.

### Reducer signature

```rust
pub fn apply(document: &Document, command: &Command) -> Result<Document, CommandError>
```

Deterministic: identical inputs always produce identical outputs.
Never mutates input. Returns cloned Document on success.

### Invariants enforced

- Document must retain at least one Profile.
- Profile must retain at least one Page.
- Missing Profile/Page/Button IDs produce typed errors.
- Deletion commands resolve entity before checking invariants.
- Button IDs are immutable (UpdateButton preserves ID).
- MoveButton fails on duplicate button ID.
- Failed commands leave the input unchanged.

### Dispatch flow (in AppState)

```
AppState::dispatch(&mut self, cmd)
    → command::apply(&self.document, cmd)
    → Ok(new_doc) → replace self.document
    → Err(e) → preserve self.document
```

Persistence is not part of the reducer. The caller is responsible for
`persist(store)` after a successful dispatch.

## Related

- TD-007: Background persistence (commands feed into the debounced writer)
- TD-008: Split PersistentState / RuntimeState (commands operate on
  PersistentState.document only)
- ADR-007: Immutable IDs (all entity references use IDs, verified by the
  reducer)
- ADR-008: Command/reducer pattern (architectural decision record)
