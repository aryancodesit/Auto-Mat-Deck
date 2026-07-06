# ADR-005: Synchronization Model

**Status:** Accepted
**Date:** 2026-07-06

## Context

Auto-Mat-Deck has two devices (mobile, desktop) that share configuration and execution state. Without a defined ownership and synchronization model, the system will face:
- Who wins when both sides modify the same configuration?
- What happens when a device is offline?
- How do we avoid data loss on reconnection?

## Decision

Adopt a **mobile-as-source-of-truth** synchronization model:

| Data | Owner | Sync |
|------|-------|------|
| User configuration (profiles, decks, triggers, workflows) | Mobile | Always Mobile → Desktop |
| Execution state (action results, context, trigger firings) | Desktop | Real-time Desktop → Mobile |
| Capabilities (available actions, plugins) | Desktop | On connect and on change |

### Rules

1. **Mobile always wins for configuration.** Desktop never modifies user-defined data. If desktop needs to change configuration (e.g., plugin requires new params), it requests the change via a capability report; mobile applies it.
2. **Desktop always wins for execution state.** Mobile reflects what the desktop reports.
3. **Full sync on connect.** Desktop receives the entire active profile when it connects or reconnects.
4. **Incremental push.** After full sync, only changes are pushed.
5. **Queue on mobile.** If desktop is offline, mobile queues changes and syncs on reconnection.
6. **No conflict resolution.** Because ownership is strictly partitioned, conflicts cannot arise.

## Rationale

- Clear, simple rules. No split-brain scenarios.
- Offline-capable by design — mobile edits don't require a live connection.
- Desktop is stateless with respect to configuration; it can be wiped and re-provisioned from mobile.

## Consequences

- Desktop must handle full re-sync gracefully (accept overwrites).
- Mobile must maintain a change queue and a sync state machine.
- Heartbeat is essential — mobile needs to know when desktop is available.
- Protocol must support both full-state and delta messages.
