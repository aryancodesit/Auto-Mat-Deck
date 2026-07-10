# ADR-006: Context is Advisory

**Status:** Accepted
**Date:** 2026-07-10

## Context

AutoMatDeck is context-aware — it knows time of day, device presence,
desktop state, and user patterns. This context could be used to suggest or
automatically execute actions.

The question is whether context should have the authority to act on its own.

## Decision

**Context recommends. The user decides.**

- Context modules can **suggest** profiles, actions, or configuration
  changes.
- User configuration remains authoritative at all times.
- Automatic execution is only permitted if the user has explicitly defined
  an automation rule that binds a context signal to an action.

```
Context
  │
  ▼
Recommended Profile / Action
  │
  ▼
User (approve / dismiss / auto-allow if rule exists)
  │
  ▼
Execution
```

## Consequences

- **Positive:** The system remains deterministic — no unexpected actions.
  Users retain full control. Automation rules are auditable and explicit.
- **Negative:** Requires a rule engine or at minimum a user-visible
  "suggestions" panel. Slightly more UI work than automatic execution.
- **Neutral:** Powers a gradual adoption path — users start with manual
  approval, add rules over time.

## Rationale

- Fully automatic context-based execution inevitably produces surprising
  behavior (phone in pocket triggers an action, timezone change causes
  unexpected schedule shift).
- Keeping the user in the loop builds trust. Once trust is established,
  users can opt into automation with explicit rules.
- This matches how home automation and IFTTT-style systems work: trigger +
  condition + action, always user-defined.
