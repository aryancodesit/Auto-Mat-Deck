# Spike ep-006: Action Execution & Process Isolation

> Validate that the desktop agent can safely execute commands before plugins exist.

## Question

Can the desktop agent execute arbitrary commands safely — with timeout, cancellation, output capture, and crash isolation?

## Scope

### Execution
- Execute executable (`.exe`, full path).
- Execute PowerShell script.
- Execute shell command (`cmd.exe` argument).
- Timeout enforcement — process killed after deadline.
- Cancellation — agent can stop a running process mid-execution.
- stdout / stderr capture and delivery to mobile.
- Exit code reporting.
- Crash isolation — one crashing action does not bring down the agent.
- Infinite loop prevention — hung process detection and kill.
- Resource limits — memory, CPU, handle count (if measurable).

### Concurrency

| Scenario | What to validate |
|----------|------------------|
| 1 command | Baseline single-execution correctness |
| 10 commands (parallel) | Agent queues and executes without interference |
| 100 commands (parallel) | Queue saturation behavior, resource limits |
| Duplicate commands | Same command submitted twice — both execute independently |
| Cancellation while queued | Queued command is removed before execution starts |
| Cancellation while running | Running process is terminated mid-execution |
| Maximum concurrent processes | Document the natural limit before degradation |
| Resource exhaustion | Run until memory/CPU limit; agent must survive |
| Ordering guarantees | Commands from same source execute in submission order |

## Out of Scope

- Plugin loading, discovery, or sandboxing (deferred to ADR-007).
- GUI automation (window handles, SendKeys).
- System-level actions requiring elevation.
- Android-side action execution.

## Success Thresholds

### Execution
| Metric | PASS | WARNING | FAIL |
|--------|------|---------|------|
| Normal execution | Returns stdout + exit code 0 | Missing output | Agent crash |
| Timeout enforcement | Process killed at deadline | Delayed kill (>2x timeout) | Process orphaned |
| Cancellation | Process killed < 1 s | Killed 1–5 s | Not killed |
| Crash isolation | Agent survives | Agent unstable | Agent crashes |
| Infinite loop | Detected and killed | Detected but not killed | Agent hangs |

### Concurrency
| Metric | PASS | WARNING | FAIL |
|--------|------|---------|------|
| 1 command | Correct output | Partial output | Error |
| 10 parallel | All complete, no cross-contamination | Some fail | Agent crash |
| 100 parallel | Agent stays responsive | Degraded throughput | Agent crash or hang |
| Cancel queued | Removed before execution | Delayed removal | Still executes |
| Cancel running | Killed < 1 s | Killed 1–5 s | Not killed |
| Ordering | In-order per source | Occasional reorder | Random order |

## Status

Planned. Not started until EP-003 (Windows lifecycle) provides a running agent.
