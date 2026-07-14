# Context Observer Architecture

**Status:** Draft — Sprint 2
**Last updated:** 2026-07-13

## Purpose

The context observer is the Win32 foreground-window polling subsystem. It
reports the current foreground process identity; observations are applied
through `DesktopRuntime` for automatic Profile resolution.

## Design constraints

- Observer must not know about rules, profiles, or projection.
- Observer produces a typed observation result — the runtime does the rest.
- Observer is `#[cfg(windows)]`. Non-Windows returns `Ok(None)`.
- Observer is not a trait. One platform, one implementation. A trait is
  added when a second platform has a committed ADR.
- Observer is synchronous and passive (polled, not event-driven).

## Module

`apps/desktop/src/observer.rs`. Top-level, flat, no sub-modules. Declared
in `main.rs` as `mod observer`.

## Observer struct

```rust
pub(crate) struct ForegroundObserver;
```

No state. No fields. Stateless means no setup, no drop, no lifecycle.

## Observation result type

```rust
#[derive(Debug)]
pub(crate) enum ContextObserverError {
    ProcessOpenFailed,        // OpenProcess or PID retrieval failed
    ProcessNameQueryFailed,   // GetModuleBaseNameW returned failure
    InvalidProcessName,       // Decoded basename was empty or invalid
    PlatformNotSupported,     // Building for non-Windows target
}

impl ForegroundObserver {
    pub(crate) fn current_context() -> Result<Option<ContextSnapshot>, ContextObserverError> {
        // ...
    }
}
```

### Win32 stage to error mapping

| Win32 stage | Observer output | Map reason |
|---|---|---|
| `GetForegroundWindow` returns `NULL` | `Ok(None)` | OS definitively reports no foreground window. |
| `GetWindowThreadProcessId` yields unusable PID | `Err(ProcessOpenFailed)` | Cannot open the process without a valid PID. Cause not classified. |
| `OpenProcess` fails | `Err(ProcessOpenFailed)` | Access denied, process exited, or handle limit — all `ProcessOpenFailed`. Cause not inspected. |
| `GetModuleBaseNameW` returns failure (0) | `Err(ProcessNameQueryFailed)` | Module-name query failed. Cause not classified. |
| Decoded basename is empty or invalid | `Err(InvalidProcessName)` | String-level domain validation. |
| Non-Windows target | `Err(PlatformNotSupported)` | Configured out at build. |

Error variants represent operational stages, not guessed OS causes. Sprint 2
does not inspect `GetLastError`.

### Semantics

| Variant | Meaning | Runtime action |
|---|---|---|
| `Ok(Some(snapshot))` | Foreground process successfully identified. | Apply observation, may update runtime. |
| `Ok(None)` | No foreground window exists (`GetForegroundWindow` returned `NULL`). | Apply observation (latest_context → None). |
| `Err(error)` | Observation failed — identity could not be obtained. | **Do not mutate runtime.** Retain previous `latest_context`. |

The `Ok(None)` path is only returned when `GetForegroundWindow` returns
`NULL` — the OS definitively reports no foreground window. All other
failures — access denied, process exit race, module-name query failure,
empty basename — are `Err`.

### Rationale

`Option<ContextSnapshot>` collapses seven distinct states into `None`,
creating a window where a transient Win32 failure (access denied, process race)
erases `latest_context` mid-transition. Consider:

```
latest_context = Some(Code.exe)
→ transient OpenProcess access denied
→ observer returns None (under old contract)
→ latest_context = None
→ next poll succeeds: Some(Code.exe)
→ latest_context = Code.exe again
```

This produces a spurious ContextChanged → Unchanged cycle that could trigger
an unnecessary active-profile re-resolution. Under the `Result` contract:

```
latest_context = Some(Code.exe)
→ OpenProcess access denied → observer returns Err
→ latest_context REMAINS Some(Code.exe)
→ active_profile_id unchanged
```

The `Err` variant is the observer's contract: "I cannot tell you what the
foreground is right now, but do not assume it changed."

### Transient failure trace

| Step | Observation | Result | `latest_context` | `active_profile_id` |
|---|---|---|---|---|
| 1 | `Code.exe` | `Ok(Some(Code.exe))` | `Some(Code.exe)` | `ProfileA` |
| 2 | `OpenProcess` access denied | `Err(ProcessOpenFailed)` | `Some(Code.exe)` (unchanged) | `ProfileA` (unchanged) |
| 3 | `Code.exe` | `Ok(Some(Code.exe))` | `Some(Code.exe)` (unchanged) | `ProfileA` (unchanged) |

## Normalized deduplication

The observer returns the executable basename as observed by the OS (e.g.
`"Code.exe"`). The runtime/domain comparison uses `normalize_process_name`.

`latest_context` retains the **observed** basename (observer adapter output),
not a canonicalized form. Normalization is applied only at comparison time
and rule-matching time.

### Four-process dedup trace

| Sequence | Observed | Normalized | Equals previous? | Transition? |
|---|---|---|---|---|
| 1 | `Code.exe` | `code.exe` | N/A (first) | Yes — first meaningful context |
| 2 | `code.exe` | `code.exe` | `code.exe` == `code.exe` | No — deduplicated |
| 3 | `CODE.EXE` | `code.exe` | `code.exe` == `code.exe` | No — deduplicated |
| 4 | `Spotify.exe` | `spotify.exe` | `spotify.exe` ≠ `code.exe` | Yes — real transition |

Deduplication comparison:

```rust
fn context_unchanged(a: &Option<ContextSnapshot>, b: &Option<ContextSnapshot>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => {
            normalize_process_name(&a.foreground_process)
                == normalize_process_name(&b.foreground_process)
        }
        (None, None) => true,
        _ => false,
    }
}
```

## RuntimeTransition structure

The old `observe_context(...) -> bool` contract is rejected. A boolean
loses the distinction between:

A. Context unchanged.
B. Context changed, active profile unchanged.
C. Context changed, active profile A → B.
D. Context changed, active profile None → A.
E. Context changed, active profile A → None.

The minimum exact fields required by Sprint 2 (and consumed by Sprint 3
projection):

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

### Why not ContextSnapshot in the transition?

Sprint 3 projection (ADR-011) only needs the resulting `active_profile_id` to
construct `ActiveProfileState`. The observer details inside `ContextSnapshot`
are not projected to Android. Copying them into the transition would expose
observer internals to the projection layer unnecessarily.

### Why previous_profile_id?

Sprint 3 push conditions (ADR-011) require sending `active_profile_state` when
active Profile changes. Without `previous_profile_id`, the projection layer
cannot distinguish "active profile changed" from "context changed but profile
stayed same" without comparing against its own cached state. The transition
is the authoritative record of what changed.

### All five cases represented

| Case | `context_changed` | `previous_profile_id` | `active_profile_id` | `active_profile_changed()` |
|---|---|---|---|---|
| A (unchanged) | `false` | `Some(A)` | `Some(A)` | `false` |
| B (context changed, profile same) | `true` | `Some(A)` | `Some(A)` | `false` |
| C (profile A → B) | `true` | `Some(A)` | `Some(B)` | `true` |
| D (profile None → A) | `true` | `None` | `Some(A)` | `true` |
| E (profile A → None) | `true` | `Some(A)` | `None` | `true` |

## DesktopRuntime mutation API

The observer calls a single orchestration method on `DesktopRuntime`. The
observer must not mutate `ProfileRuntime` directly.

Error handling lives in the polling worker **before** the runtime call. The
method only accepts a successfully observed context (`Option<ContextSnapshot>`,
never `Err`):

```rust
impl DesktopRuntime {
    /// Apply a successful foreground observation.
    /// Caller must pass Ok(Some) or Ok(None); Err is handled at the poll level.
    pub(crate) fn apply_context_observation(
        &mut self,
        snapshot: Option<ContextSnapshot>,
    ) -> RuntimeTransition {
        let prev_profile = self.runtime.active_profile_id.clone();
        let prev_ctx = self.runtime.latest_context.clone();

        let ctx_unchanged = match (&prev_ctx, &snapshot) {
            (Some(a), Some(b)) => {
                normalize_process_name(&a.foreground_process)
                    == normalize_process_name(&b.foreground_process)
            }
            (None, None) => true,
            _ => false,
        };

        if ctx_unchanged {
            return RuntimeTransition {
                context_changed: false,
                previous_profile_id: prev_profile,
                active_profile_id: self.runtime.active_profile_id.clone(),
            };
        }

        self.runtime.latest_context = snapshot;

        if self.runtime.selection_mode == SelectionMode::Automatic {
            self.runtime.active_profile_id = resolve_active_profile(
                &self.app.document.profiles,
                &self.app.document.context_rules,
                self.runtime.latest_context.as_ref(),
                &self.runtime.selection_mode,
            );
        }

        RuntimeTransition {
            context_changed: true,
            previous_profile_id: prev_profile,
            active_profile_id: self.runtime.active_profile_id.clone(),
        }
    }
}
```

Key properties:

- `ctx_unchanged` uses `normalize_process_name` for comparison (case-insensitive dedup).
- `latest_context` is only updated on a meaningful change — not on dedup, not on error.
- In Manual mode, `latest_context` is still recorded (so it's available when
  user switches back to Automatic).
- `resolve_active_profile` is only invoked when mode is Automatic and context
  meaningfully changed.
- The transition captures both `context_changed` and profile transition for
  Sprint 3 projection.

### Context + profile dual-change semantics

When context changes and the rule resolves to a different Profile, both
`context_changed = true` and `active_profile_changed() = true` in the same
transition. The structured struct represents both dimensions simultaneously —
no single-variant enum can, which is why a mutually-exclusive `enum` was
rejected.

## Win32 API final decision

`GetModuleBaseNameW` is chosen over `QueryFullProcessImageNameW`.

### Comparison

| Criterion | `GetModuleBaseNameW` | `QueryFullProcessImageNameW` |
|---|---|---|
| Access required | `PROCESS_QUERY_INFO \| PROCESS_VM_READ` | `PROCESS_QUERY_LIMITED_INFO` |
| Output | Basename only: `"Code.exe"` | Full path: `"C:\Program Files\Code.exe"` |
| Path parsing | None | Must extract basename |
| Feature flag | `Win32_System_ProcessStatus` | Already present |

### Decision

`GetModuleBaseNameW` wins because:

1. It returns exactly what the domain needs (the basename). No path parsing,
   no `Path::file_name()`, no extra `unsafe` to split the string.
2. The access flags are a superset of the minimum — `PROCESS_VM_READ` is
   required by `GetModuleBaseNameW` but does not introduce a meaningful
   security difference for querying the current user's own processes.
3. On access denied (lock screen / UAC desktop / elevated process), both APIs
   fail identically — access rights is not a distinguishing factor.
4. One fewer Win32 call in the hot path.

### `windows-sys` feature audit

Current `Cargo.toml` features (windows-sys 0.59):

| Feature | Already present | Required by observer |
|---|---|---|
| `Win32_Foundation` | Yes | `HWND`, `HANDLE`, `INVALID_HANDLE_VALUE` |
| `Win32_System_Threading` | Yes | `OpenProcess`, `CloseHandle` |
| `Win32_UI_WindowsAndMessaging` | Yes | `GetWindowThreadProcessId` |
| `Win32_UI_WindowstationAndDesktop` | **No** | `GetForegroundWindow` |
| `Win32_System_ProcessStatus` | **No** | `GetModuleBaseNameW` |

Two feature flags must be added during Sprint 2 implementation:
`Win32_UI_WindowstationAndDesktop`, `Win32_System_ProcessStatus`.

## Polling

### Ownership

A dedicated `std::thread` spawned in `main.rs`. Not on the Tokio runtime.
Matches the existing tray-thread pattern.

### Interval

200 ms (`Duration::from_millis(200)`). Approximately 5 context checks per
second. Fast enough to feel instant to a human; slow enough to avoid
measurable CPU from repeated `GetForegroundWindow` calls.

### Terminology

A 200 ms polling interval is **rate limiting**, not temporal debounce.

- **Rate limiting:** Maximum observation frequency is bounded by the poll
  interval. This prevents the observer from consuming CPU on every window
  event.
- **Debounce:** A stability window that waits for a signal to settle before
  emitting. Debounce is not implemented in Sprint 2.

**Sprint 2 transition policy:**

1. Normalized-equivalent observations are deduplicated (string comparison).
2. Repeated identical observations produce no transition.
3. Rapidly alternating real processes (e.g. `Code.exe` ↔ `Spotify.exe` every
   100 ms) still produce transitions at poll frequency, because they are
   meaningfully different after normalization.
4. Temporal debounce / stability windows are deferred until hardware or manual
   UX testing proves the naive policy causes observable flicker in
   `active_profile_state` messages.

### Thread body (pseudocode)

```rust
fn observer_thread(
    runtime: SharedRuntime,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
) {
    // Wait for initial context before entering loop (avoids immediate None)
    std::thread::sleep(Duration::from_millis(200));

    loop {
        // 1. Shutdown check (synchronous, no Tokio needed)
        if shutdown_rx.has_changed().unwrap_or(false) && *shutdown_rx.borrow() {
            break;
        }

        // 2. Win32 observation WITHOUT runtime lock
        let observation = ForegroundObserver::current_context();

        // 3. Handle error — log, do not mutate runtime
        let snapshot = match observation {
            Ok(snapshot) => snapshot,
            Err(e) => {
                // Rate-limit log: warn once, info subsequently
                // (log implementation detail in Sprint 2)
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }
        };

        // 4. Acquire lock, apply observation
        let transition = {
            let mut guard = runtime.lock().unwrap();
            guard.apply_context_observation(snapshot)
        };

        // 5. Release lock before any side effects

        // 6. Future Sprint 3: examine transition outside lock,
        //    send active_profile_state if active_profile_changed()
        // if transition.active_profile_changed() {
        //     agent::broadcast_active_profile(&transition.active_profile_id, &runtime);
        // }

        // Synchronise rate even when no meaningful work
        std::thread::sleep(Duration::from_millis(200));
    }
}
```

### Lock discipline

1. Win32 observation: **no lock held**.
2. If `Err`, log and skip — **no lock required**.
3. Acquire `SharedRuntime` lock.
4. Call `apply_context_observation` (updates `latest_context`, resolves profile).
5. Capture `RuntimeTransition`.
6. Release lock.
7. Examine transition outside lock for Sprint 3 projection.
8. Do not perform WebSocket sends under `SharedRuntime` lock.

This guarantees minimum lock hold time (<1 µs per observation) and prevents
lock contention with the GUI and agent threads.

## Manual mode lifecycle trace

### Initial state

```
selection_mode = Manual(ProfileA)
active_profile_id = ProfileA
latest_context = None
```

### Observe Code.exe (no matching rule)

```
ForegroundObserver → Ok(Some(Code.exe))
apply_context_observation:
  context_changed = true
  latest_context = Some(Code.exe)
  selection_mode = Manual(ProfileA)
  resolve_active_profile not called (Manual mode → skipped)
  active_profile_id = ProfileA (unchanged)
RuntimeTransition { context_changed: true, previous: A, active: A }
```

### Observe Spotify.exe (rule maps to ProfileB)

```
ForegroundObserver → Ok(Some(Spotify.exe))
apply_context_observation:
  context_changed = true
  latest_context = Some(Spotify.exe)
  selection_mode = Manual(ProfileA)
  resolve_active_profile not called
  active_profile_id = ProfileA (unchanged)
RuntimeTransition { context_changed: true, previous: A, active: A }
```

### ProfileA deleted (document reconciliation)

`ProfileRuntime::reconcile` runs (Sprint 1, `model.rs:197`):

1. Detects `Manual(ProfileA)` is stale → sets `selection_mode = Automatic`
2. Detects `active_profile_id = Some(ProfileA)` points to deleted profile
3. Calls `resolve_active_profile` with retained `latest_context = Some(Spotify.exe)`
4. Rule maps `Spotify.exe` → `ProfileB`
5. Result: `selection_mode = Automatic`, `active_profile_id = ProfileB`

The self-healing is provided by `ProfileRuntime::reconcile` (Sprint 1
implementation at `model.rs:197-223`), which was already designed for this.

## Shutdown

The observer thread receives a `tokio::sync::watch::Receiver<bool>` — same
type as the existing agent thread (`agent.rs:31`).

**Recommended check** (synchronous, no Tokio required):

```rust
if shutdown_rx.has_changed().unwrap_or(false) && *shutdown_rx.borrow() {
    break;
}
```

`Receiver::has_changed()` is a synchronous call that returns `Ok(true)` if
a new value is available since the last call. This works in any thread
context. The `unwrap_or(false)` handles the closed-channel case (all senders
dropped) gracefully.

The existing `main.rs` already clones the receiver for each spawned thread:

```rust
let shutdown_rx_for_observer = shutdown_rx_from_main.clone();
```

This follows the same pattern as `shutdown_rx_for_server` (`main.rs:62`).

## Non-Windows platform

```rust
#[cfg(not(windows))]
impl ForegroundObserver {
    pub(crate) fn current_context() -> Result<Option<ContextSnapshot>, ContextObserverError> {
        Err(ContextObserverError::PlatformNotSupported)
    }
}
```

The observer thread spawn in `main.rs` is `#[cfg(windows)]` gated.

## Testing

### Pure deterministic tests (no Win32 calls, no mocking)

| # | Test intention | Input | Expected result |
|---|---|---|---|
| 1 | Normalized equivalent observation deduplicates | `Ok(Some(Code.exe))` then `Ok(Some(code.exe))` | `context_changed = false` |
| 2 | Substring-different process is a real transition | `Ok(Some(spotify.exe))` then `Ok(Some(spotify))` | `context_changed = true` |
| 3 | Process transition updates `latest_context` | `Ok(Some(Code.exe))` → `Ok(Some(Spotify.exe))` | `latest_context = Spotify.exe` |
| 4 | Automatic matching rule changes active profile | `Ok(Some(Code.exe))`, rule `code.exe → ProfileB` | `active_profile_id = ProfileB` |
| 5 | Automatic no-match falls back to first Profile | `Ok(Some(unknown.exe))`, no matching rule | `active_profile_id = first profile` |
| 6 | Manual mode retains active profile | Manual(ProfileA), observe `Ok(Some(Code.exe))` | `active_profile_id = ProfileA` |
| 7 | Manual mode records `latest_context` | Manual(ProfileA), observe `Ok(Some(Code.exe))` | `latest_context = Some(Code.exe)` |
| 8 | Stale Manual deletion uses retained `latest_context` | Stale Manual, `latest_context = Some(Spotify.exe)` with rule → ProfileB | `active_profile_id = ProfileB` via `reconcile` |
| 9 | Observer error leaves `latest_context` unchanged | `Err(ProcessOpenFailed)` | `latest_context` unchanged |
| 10 | Observer error leaves `active_profile_id` unchanged | `Err(ProcessOpenFailed)` | `active_profile_id` unchanged |
| 11 | Context changed, profile unchanged | Manual(ProfileA), observe `Ok(Some(Code.exe))` | `context_changed = true`, `active_profile_changed() = false` |
| 12 | Context changed, profile A → B | Automatic, rule maps new context to ProfileB | `context_changed = true`, `active_profile_changed() = true` |
| 13 | Profile transition None → A | First observation, rule maps to ProfileA | `active_profile_id = ProfileA` |
| 14 | Profile transition A → None | Rule deleted, no profiles remain | `active_profile_id = None` |
| 15 | No Profiles resolves None | Empty profiles, any observation | `active_profile_id = None` |
| 16 | Win32 adapter remains isolated | Only tests on `apply_context_observation` with synthetic `Option<ContextSnapshot>` | No Win32 calls in domain tests |

### Windows manual / integration validation (not in CI)

| # | Scenario |
|---|---|
| 17 | Switch between two known applications — verify log output shows context change |
| 18 | Lock workstation (Win+L) — verify `latest_context` not erased on transient access-denied |
| 19 | Close foreground application while observer is running — verify graceful `Ok(None)` or `Err` |
| 20 | Rapid Alt+Tab between two windows — verify no spurious projection storms |

### Implementation note

`apply_context_observation` is a pure function of `(DesktopRuntime, Option<ContextSnapshot>) → RuntimeTransition`. Every domain test instantiates a `DesktopRuntime` directly with the relevant `AppState` and `ProfileRuntime` configuration. No Win32 calls, no threading, no shared state.
