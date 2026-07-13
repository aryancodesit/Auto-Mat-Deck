# ADR-012: Foreground Context Observation Boundary

**Status:** Accepted
**Date:** 2026-07-13

## Context

v0.3 introduces a foreground-process context observer. The observer must
identify the current Windows foreground process; observations are applied
through `DesktopRuntime` for automatic Profile resolution.

The question is: what is the observer's responsibility boundary, and how does
it communicate observations to the runtime?

## Decision

### Observer responsibility

The observer's sole responsibility is OS-level foreground process
identification. It must not know about rules, profiles, projection, or
runtime state.

### Synchronous passive observer

The observer is synchronous (polled, not event-driven). It does not own a
thread, does not manage timers, and does not implement temporal debounce.
Polling interval, shutdown, and rate-limiting are the caller's
responsibility (the poll worker thread).

### No observer-owned thread

`ForegroundObserver` is a stateless struct with a single static method. The
poll worker thread is owned by `main.rs` (the application bootstrap), not
by the observer. This keeps the observer testable without threading.

### Result type over Option

`Option<ContextSnapshot>` is rejected as the return type because it collapses
seven distinct states into `None`, including transient Win32 failures that
should not erase retained context.

The observer returns:

```rust
pub(crate) fn current_context() -> Result<Option<ContextSnapshot>, ContextObserverError>
```

Semantics:

| Variant | Meaning |
|---|---|
| `Ok(Some(snapshot))` | Foreground process successfully identified. |
| `Ok(None)` | No foreground window exists (OS definitively reports none). |
| `Err(error)` | Observation failed — runtime state must remain unchanged. |

### Exact error variants

```rust
pub(crate) enum ContextObserverError {
    ProcessOpenFailed,        // OpenProcess or PID retrieval failed
    ProcessNameQueryFailed,   // GetModuleBaseNameW returned failure
    InvalidProcessName,       // Decoded basename was empty or invalid
    PlatformNotSupported,     // Building for non-Windows target
}
```

### Win32 stage to error mapping

| Win32 stage | Observer output | Map reason |
|---|---|---|
| `GetForegroundWindow` returns `NULL` | `Ok(None)` | OS definitively reports no foreground window. |
| `GetWindowThreadProcessId` yields unusable PID | `Err(ProcessOpenFailed)` | Cannot open the process without a valid PID. |
| `OpenProcess` fails | `Err(ProcessOpenFailed)` | Access denied, process exited, or handle limit — all `ProcessOpenFailed`. |
| `GetModuleBaseNameW` returns failure (0) | `Err(ProcessNameQueryFailed)` | Module-name query failed. |
| Decoded basename is empty or invalid | `Err(InvalidProcessName)` | String-level domain validation. |
| Non-Windows target | `Err(PlatformNotSupported)` | Configured out at build. |

Error variants represent operational stages, not guessed OS causes. Sprint 2
does not inspect `GetLastError`.

### Win32 API choice

`GetModuleBaseNameW` is used over `QueryFullProcessImageNameW`.

- `GetModuleBaseNameW` returns the executable basename (`"Code.exe"`)
  directly — no path parsing, no `Path::file_name()`.
- Required access: `PROCESS_QUERY_INFORMATION | PROCESS_VM_READ` (a superset
  of `PROCESS_QUERY_LIMITED_INFORMATION` but functionally equivalent for
  querying the current user's own processes).
- On access denied (lock screen, UAC secure desktop, elevated process), both
  APIs fail identically.
- One fewer Win32 call in the hot path.

### Executable basename contract

The observer returns the basename exactly as reported by the OS (e.g.
`"Code.exe"`). The observer does NOT normalize. Normalization
(`normalize_process_name`: trim, lowercase) is a domain responsibility
applied at comparison time by the runtime.

### Trait decision

A trait is not introduced for the observer. The `ForegroundObserver` struct
is called directly by the poll worker. A trait is added when a second
platform (macOS, Linux) has a committed ADR with a concrete implementation
plan.

### Direct Windows implementation

On Windows, the observer calls Win32 APIs directly through `windows-sys`.
On non-Windows, `current_context()` returns
`Err(ContextObserverError::PlatformNotSupported)`.

### windows-sys features

Two feature flags must be added to `Cargo.toml` during Sprint 2
implementation:

- `Win32_UI_WindowstationAndDesktop` — `GetForegroundWindow`
- `Win32_System_ProcessStatus` — `GetModuleBaseNameW`

`OpenProcess` and `CloseHandle` are already available under
`Win32_System_Threading` (present). `GetWindowThreadProcessId` is available
under `Win32_UI_WindowsAndMessaging` (present).

## Alternatives considered

### SetWindowsHookEx (event-driven)

Rejected. A global CBT hook would receive window-activation events without
polling, but requires a message pump, complicates the threading model, and
introduces DLL-injection concerns for a 64-bit-only process. The polling
approach at 200 ms provides equivalent UX with simpler code.

### QueryFullProcessImageNameW

Rejected in favour of `GetModuleBaseNameW`. See Win32 API choice above.

### Trait-based observer

Rejected until a second platform has a committed ADR. YAGNI applies.

## Consequences

- **Positive:** Observer is testable without Win32 mocking (domain tests use
  synthetic `Option<ContextSnapshot>` directly on
  `apply_context_observation`).
- **Positive:** `Err` retention policy prevents transient failures from
  erasing `latest_context`.
- **Positive:** Stateless observer means zero setup, zero lifecycle.
- **Neutral:** Two `windows-sys` feature flags must be added during
  implementation.
- **Negative:** Polling at 200 ms consumes CPU even when foreground is idle.
  Mitigated: `GetForegroundWindow` is a fast kernel call; 200 ms is
  approximately 5 calls/second, negligible on modern hardware.
