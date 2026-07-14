# ADR-003: Native execution over shell execution

**Status:** Accepted
**Date:** 2026-07-10

## Context

Actions on the Desktop (launch, lock, toast, key input) could be executed
via shell interpreters (cmd.exe, PowerShell) or via native Windows APIs.

## Decision

All remote actions use **native Windows APIs directly**. No shell
interpreter is invoked in the execution path.

## Consequences

- **Positive:** No shell injection risk; no dependency on cmd or PowerShell
  being present; cleaner error handling; faster execution.
- **Negative:** Slightly more code per action; limited to actions that have
  a native API (no arbitrary shell commands).

## Rationale

- Security: Shell interpreters are a common vector for injection attacks.
  Even with input sanitization, the risk is non-zero.
- Reliability: PowerShell cmdlets change between Windows versions; native
  APIs are more stable.
- Auditability: The code path for each action is explicit and bounded.
- If a future use case requires arbitrary command execution, it will be
  added as a specific opt-in action with its own sandboxing.
