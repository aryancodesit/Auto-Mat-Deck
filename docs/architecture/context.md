# Context Architecture

**Status:** Draft — v0.3 context foundation
**Last updated:** 2026-07-13

## Purpose

v0.3 introduces foreground-process context awareness. The desktop observes the
Windows foreground window, resolves the observed process against user-defined
rules, and activates the corresponding Profile. The active Profile definition
is projected to connected Android clients for dynamic deck rendering.

## Concepts

```
┌──────────────────────────────────────────────────────┐
│                    ProfileRuntime                     │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │ active       │  │ selection    │  │ latest      │ │
│  │ profile_id   │  │ mode         │  │ context     │ │
│  └──────────────┘  └──────────────┘  └────────────┘ │
└──────────────────────┬───────────────────────────────┘
                       │
         ┌─────────────┴──────────────┐
         │     ActiveProfileResolver  │  (pure function)
         └─────────────┬──────────────┘
                       │
         ┌─────────────┴──────────────┐
         │  ContextSnapshot  + Rules  │
         └────────────────────────────┘
```

### ContextSnapshot

The minimum observed environment state for v0.3. Foreground process only.

```rust
pub struct ContextSnapshot {
    pub foreground_process: String,   // e.g. "code.exe" (as reported by observer)
}
```

### ContextRule

A user-defined mapping from a normalized process name to a Profile.

```rust
pub struct ContextRule {
    pub id: ContextRuleId,
    pub process_name: String,         // stored normalized (trimmed, lowercased)
    pub profile_id: ProfileId,
}
```

ContextRules belong to `Document`, not `Profile`. See ADR-009.

### SelectionMode

Controls how the active Profile is determined.

```rust
pub enum SelectionMode {
    Automatic,
    Manual(ProfileId),
}
```

- **Automatic:** The `ActiveProfileResolver` determines the active Profile
  based on `ContextSnapshot` + `ContextRule` + available Profiles.
- **Manual(ProfileId):** The user has explicitly selected a Profile. Automatic
  context resolution does not replace it.

### ProfileRuntime

The authoritative runtime state. Not persisted. Not part of the Command/reducer
pipeline.

```rust
pub struct ProfileRuntime {
    pub active_profile_id: Option<ProfileId>,
    pub selection_mode: SelectionMode,
    pub latest_context: Option<ContextSnapshot>,
}
```

### ActiveProfileResolver

A pure function that determines the active Profile based on current context,
rules, and selection mode. No side effects — no Win32 calls, no WebSocket
messages, no GUI mutations.

```rust
pub fn resolve_active_profile(
    rules: &[ContextRule],
    profiles: &[Profile],
    snapshot: &ContextSnapshot,
    mode: &SelectionMode,
) -> Option<ProfileId>
```

## Resolution policy

### Automatic mode

1. Resolver receives `ContextSnapshot` + `Document.context_rules` + `Document.profiles`.
2. Matches `snapshot.foreground_process` against rules using `normalize_process_name`.
3. **Exact match found:** Returns the matched `ProfileId`.
4. **No match:** Falls back to `profiles.first().map(|p| p.id.clone())`.
5. **No Profiles:** Returns `None` — no active Profile.

### Manual mode

1. If the stored `Manual(ProfileId)` is valid (the Profile still exists in
   Document), return it. Context is ignored.
2. If the stored `Manual(ProfileId)` is stale (Profile was deleted), the
   manual selection is invalidated and the mode falls back to `Automatic`.
   The resolver runs in automatic mode.

### Stale manual policy

**Accepted:** A stale `Manual(ProfileId)` invalidates manual selection and
reverts to `Automatic` resolution. The runtime's `active_profile_id` is
re-resolved by the resolver.

Alternative considered: hold at `None` until user intervenes. Rejected: the
system should self-heal. If the user's manually selected Profile is deleted,
the first available Profile is better than a blank deck.

## Process normalization

One domain function:

```rust
pub fn normalize_process_name(name: &str) -> String {
    name.trim().to_lowercase()
}
```

Used by:

- **ContextRule insertion** — Normalize before storage and duplicate checking.
- **ActiveProfileResolver matching** — Normalize both rule `process_name` and
  snapshot `foreground_process` before comparison.

The Windows observer does NOT normalize. It reports the process identity as
observed by the OS (e.g. `"Code.exe"`). Normalization is a domain
responsibility.

## Matching semantics

- Trimmed
- Case-insensitive exact match
- No substring matching
- No regex
- No glob patterns

Examples:

| Observed process | Rule process_name | Match |
|---|---|---|
| `Code.exe` | `code.exe` | Yes |
| `CHROME.EXE` | `chrome.exe` | Yes |
| `chrome.exe` | `chrome` | No (substring) |
| `code.exe` | `mycode.exe` | No |
| `Code.exe` | ` code.exe ` | Yes (trimmed) |

## Focus stabilization

The Windows observer does not immediately emit a new `ContextSnapshot` on every
foreground window change. Instead it applies a stability window:

```rust
const FOREGROUND_STABILITY_MS: u64 = 250;
```

The observer maintains:

- `candidate_process: Option<String>` — the current foreground process identity
- `candidate_since: Option<Instant>` — when this process was first observed

A new `ContextSnapshot` is emitted only when the same process is observed
continuously for `FOREGROUND_STABILITY_MS`.

The resolver contains no timers. Timing is solely the observer's responsibility.

## Runtime reconciliation

ProfileRuntime is reconciled after every Document mutation (Command dispatch).
Because both `AppState` and `ProfileRuntime` live under the same
`Arc<Mutex<DesktopRuntime>>`, the reconciliation happens within the same lock
scope:

1. Lock `DesktopRuntime`.
2. Mutate `self.app.document` via `dispatch()` (the reducer).
3. Call `self.runtime.reconcile(&self.app.document)`.
4. If the active Profile is stale, re-run the resolver.
5. Unlock.

The `reconcile` method is:

```rust
impl ProfileRuntime {
    pub fn reconcile(&mut self, doc: &Document) {
        match &self.selection_mode {
            SelectionMode::Manual(pid) => {
                if !doc.profiles.iter().any(|p| p.id == *pid) {
                    // Stale manual selection — fall back to automatic
                    self.selection_mode = SelectionMode::Automatic;
                }
            }
            SelectionMode::Automatic => {
                // Re-resolved when context changes, not here.
                // This only ensures active_profile_id remains valid.
            }
        }
        // If active_profile_id points to a deleted profile, clear it.
        if let Some(ref pid) = self.active_profile_id {
            if !doc.profiles.iter().any(|p| p.id == *pid) {
                self.active_profile_id = None;
            }
        }
    }
}
```

## Editor/runtime invariant

`EditorUi::selected_profile_id` and `ProfileRuntime::active_profile_id` are
completely separate concepts. This invariant is release-blocking:

- Context switching (foreground change → ProfileRuntime update) must never
  change `EditorUi::selected_profile_id`.
- Clicking a Profile in the desktop editor must never change
  `ProfileRuntime::active_profile_id`.

These two paths are orthogonal. The editor shows what the user is editing.
The runtime determines what Android renders. They can be the same Profile
by coincidence, never by coupling.

## Foreground observer design

The Windows foreground observer follows this Win32 API chain:

1. `GetForegroundWindow()` → `HWND`
2. `GetWindowThreadProcessId(hwnd, &pid)` → `DWORD pid`
3. `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid)` → `HANDLE`
4. `QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, buffer, &size)` → path
5. Extract basename: `"C:\Program Files\Code.exe"` → `"Code.exe"`
6. `CloseHandle(handle)`

Edge cases:

| Condition | Behaviour |
|---|---|
| `GetForegroundWindow` returns `NULL` | No candidate, no emission |
| PID lookup fails | Transient — no emission |
| Process exits during query | OpenProcess fails — no emission |
| Empty process name | No emission |
| Lock screen / UAC / secure desktop | `GetForegroundWindow` returns a handle but query access may fail — no emission |
| Repeated same process | Stability counter accumulates; emit after 250ms |

The observer is `#[cfg(windows)]` guarded. On non-Windows platforms,
`poll()` always returns `None`.
