# Profiles

> Profiles define the user's configuration and device bindings.

## Concept

A **Profile** is a named set of configuration that includes:
- Device pairings
- Active decks
- Trigger bindings
- Context mappings
- Plugin preferences

## Lifecycle

1. **Created** — User defines a new profile on mobile
2. **Activated** — Profile is synced to desktop agent
3. **Modified** — Changes are pushed from mobile to desktop in real-time
4. **Deleted** — Profile is removed from both devices

## Multi-Profile Support

- Users can maintain multiple profiles for different contexts (home, work, studio).
- Only one profile is active at a time.
- Profile switching triggers a reconfiguration of the desktop agent.
