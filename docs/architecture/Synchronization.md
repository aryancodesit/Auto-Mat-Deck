# Synchronization

> How state is synchronized between mobile and desktop devices.

## Ownership Model

| Data | Owner | Sync Direction |
|------|-------|----------------|
| Profiles | Mobile | Mobile → Desktop |
| Decks | Mobile | Mobile → Desktop |
| Triggers | Mobile | Mobile → Desktop |
| Actions | Desktop (via plugins) | Desktop → Mobile (capability report) |
| Workflows | Mobile | Mobile → Desktop |
| Context | Desktop | Desktop → Mobile (real-time events) |
| Plugin configuration | Desktop | Desktop → Mobile (capability-driven UI) |

**Mobile is the source of truth** for user-defined configuration. The desktop agent is a stateless executor that reflects the mobile's intent.

## Sync Mechanisms

| Mode | Trigger | Description |
|------|---------|-------------|
| Full sync | On connect / pairing | Desktop receives the complete active profile and all associated configuration |
| Incremental push | On change (mobile) | Mobile pushes only the changed entity (profile field, trigger toggle, etc.) |
| State report | On change (desktop) | Desktop pushes execution state, context updates, and trigger firings |
| Heartbeat | Periodic | Desktop confirms it is alive and reports basic status |

## Offline Behavior

- Mobile works fully offline for configuration editing. Changes are queued.
- Desktop continues executing the last synced profile when mobile is disconnected.
- When mobile reconnects, it syncs its queued changes and reconciles with desktop state.
- No conflict resolution is needed — mobile always wins for configuration; desktop always wins for execution state.

## Consistency Guarantees

- **Eventual consistency** — mobile and desktop may be briefly out of sync.
- **At-least-once delivery** for configuration pushes (mobile retries until acknowledged).
- **Best-effort ordering** — desktop processes messages in arrival order; mobile should send dependent changes in sequence.
