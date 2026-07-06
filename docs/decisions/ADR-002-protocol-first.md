# ADR-002: Protocol-First Development

**Status:** Proposed
**Date:** 2026-07-06

## Context

The mobile app and desktop agent must communicate over a shared protocol. If each application defines its own communication layer independently, they will inevitably diverge, leading to integration issues.

## Decision

Define the communication protocol (message types, wire format, sequencing) as the first concrete deliverable, before either application is implemented.

- The protocol lives in `packages/protocol/`
- Both mobile and desktop depend on this package
- The protocol is versioned independently if needed

## Rationale

- Enables parallel development of mobile and desktop against a fixed contract
- Catches design issues early, when they're cheap to fix
- Protocol can be tested in isolation before either app exists
- Forces clarity about what each side sends and expects

## Consequences

- Initial development velocity may be slower (protocol definition takes time)
- Protocol changes after development has started must be coordinated across both apps
- Requires buy-in from both mobile and desktop developers
