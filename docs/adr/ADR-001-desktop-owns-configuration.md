# ADR-001: Desktop owns configuration

**Status:** Accepted
**Date:** 2026-07-10

## Context

AutoMatDeck has two components (Desktop, Mobile) that need to agree on
which devices are paired and what actions are available. A decision is
needed about where configuration authority lives.

## Decision

The **Desktop** is the single source of truth for all configuration:

- Paired device registry
- Action definitions and permissions
- Desktop display name

Mobile stores nothing beyond a local cache of discovered Desktops.

## Consequences

- **Positive:** No sync conflicts; Mobile is stateless from a config
  perspective; pairing only requires Desktop UI for confirmation.
- **Negative:** Desktop must be running for any new device to pair;
  Mobile cannot operate fully offline.

## Rationale

- Desktop is always-on (or can be) and has a persistent filesystem.
- Mobile is transient and may be wiped/replaced.
- A single authority avoids split-brain scenarios.
- This is the simplest correct model for a personal tool with ≤5 devices.
