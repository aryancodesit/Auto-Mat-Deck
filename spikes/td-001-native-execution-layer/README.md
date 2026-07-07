# TD-001 — Native Execution Layer

**Status**: ✅ Implemented and verified

**Origin**: Security review of EP-003 spike identified two shell interpreter dependencies.

## Problem

The EP-003 implementation uses three interpreters in the execution path:

```
Rust → cmd.exe /C start → target   (launch, open_url, open_file)
Rust → powershell.exe -Command      (notify)
Rust → LockWorkStation()            (lock — clean)
```

This introduces:
1. Shell injection vectors (`&`, `&&`, `|`, `||`, `>`, `<`) via `cmd /C`
2. A PowerShell dependency for notifications (another interpreter, another surface)
3. Inconsistent error handling (cmd.exe errors are lost, PowerShell stderr is fragile)

The phone is trusted — this is not an external attacker concern. The architectural violation is that the execution engine delegates parsing to a shell rather than executing actions directly.

## Scope

Replace every shell interpreter in the execution path while preserving:

- The `Action` trait interface unchanged
- The `ActionRegistry` dispatch unchanged
- The WebSocket protocol unchanged
- The Android client unchanged

### Specific replacements

| Current | Target | Implemented |
|---------|--------|-------------|
| `cmd /C start "" <app>` | `ShellExecuteW` (`open` verb) | ✅ `shell_execute()` helper in `actions.rs` |
| `cmd /C start "" <url>` | `ShellExecuteW` (`open` verb) | ✅ `shell_execute()` helper in `actions.rs` |
| `cmd /C start "" <path>` | `ShellExecuteW` (`open` verb) | ✅ `shell_execute()` helper in `actions.rs` |
| `powershell.exe -Command <toast>` | `winrt-notification` crate (WinRT) | ✅ `show_windows_toast()` in `actions.rs` |

## Verification

All 5 actions verified with native implementations **2026-07-07**:

| Action | Implementation | Result |
|--------|---------------|--------|
| `launch chrome` | `ShellExecuteW("open", "chrome")` | ✅ Chrome opened via App Paths registry |
| `open_url github` | `ShellExecuteW("open", "https://...")` | ✅ Default browser opened |
| `open_file calc` | `ShellExecuteW("open", "C:\Windows\...\calc.exe")` | ✅ Calculator launched |
| `lock` | `LockWorkStation()` | ✅ WorkStation locked |
| `notify` | `winrt_notification::Toast` | ✅ Windows toast notification appeared |

Execution path after TD-001:

```
Android → Rust → Win32 API (ShellExecuteW / LockWorkStation / winrt-notification) → Target
```

No `cmd.exe`, `powershell.exe`, or shell interpreters in the action path.

## Non-goals

- No protocol changes
- No Android changes
- No new actions
- No Action trait redesign
- No shell execution for any purpose
