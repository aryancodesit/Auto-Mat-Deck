# TD-001 — Native Execution Layer

**Status**: 📋 Planned (v0.2)

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

| Current | Target |
|---------|--------|
| `cmd /C start "" <app>` | `std::process::Command::new(app).spawn()` or `CreateProcessW` |
| `cmd /C start "" <url>` | `ShellExecuteExW` with `open` verb |
| `cmd /C start "" <path>` | `ShellExecuteExW` with `open` verb |
| `powershell.exe -Command <toast>` | Native WinRT toast via `windows` crate or tray-icon notification API |

## Verification

After each replacement, all 5 EP-003 actions must still pass:

```
launch chrome    → chrome.exe opens
open_url github  → browser opens
open_file calc   → Calculator opens
lock             → WorkStation locks
notify           → toast appears
```

## Non-goals

- No protocol changes
- No Android changes
- No new actions
- No Action trait redesign
- No shell execution for any purpose
