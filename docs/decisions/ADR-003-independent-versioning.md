# ADR-003: Independent Versioning

**Status:** Accepted
**Date:** 2026-07-06

## Context

The initial design coupled all three components (mobile, desktop, protocol) to a single shared version number. This creates problems:
- A protocol change forces a version bump for both apps even if they didn't change.
- Mobile and desktop evolve at different cadences (mobile releases are gated by app store review; desktop can ship whenever).
- Protocol version is meaningful to compatibility; app version is meaningful to users.

## Decision

Decouple versioning:

| Component | Version Example | Granularity |
|-----------|----------------|-------------|
| Mobile | `1.2.0` | Per Android release |
| Desktop | `1.4.1` | Per agent release |
| Protocol | `2.0` | Per breaking/non-breaking schema change |

Desktop advertises supported protocol versions during discovery and pairing:

```json
{
  "deviceId": "abc-123",
  "protocolVersions": ["1.2", "1.3", "2.0"]
}
```

Mobile selects the highest mutually supported version during handshake.

## Rationale

- Allows independent release cycles without artificial synchronization.
- Protocol can mature independently of application code.
- Users see meaningful version numbers for each component.
- Backward compatibility is explicit — desktop can support multiple protocol versions.

## Consequences

- CI must track three separate version strings.
- Protocol version negotiation logic must be implemented during handshake.
- Release notes must clearly state which protocol versions each app release supports.
- Adding a version negotiation field to the discovery announcement is required.
