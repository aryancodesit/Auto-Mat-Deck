# Spike ep-007: Protocol Evolution & Version Compatibility

> Validate that the protocol can evolve without breaking existing clients.

## Question

Can a v1 mobile talk to a v2 desktop? Can new fields be added without breaking old clients? Can commands be deprecated gracefully?

## Scope

- Version negotiation during handshake (mobile and desktop advertise supported versions).
- Backward compatibility: v1 client sends message to v2 server → v2 responds correctly.
- Forward compatibility: v2 client sends message with new field to v1 server → v1 ignores unknown field.
- Optional field handling — missing field does not break deserialization.
- Unknown message type — server receives a message type it does not recognize → graceful rejection.
- Deprecated command — old client sends deprecated command → server responds with deprecation warning.
- Field type change — breaking vs. non-breaking changes identified.
- Version downgrade — high-version client connects to low-version server → negotiation picks common version.

## Out of Scope

- Transport or encryption (covered by EP-001).
- Performance benchmarking.
- Plugin API versioning.

## Success Thresholds

| Metric | PASS | WARNING | FAIL |
|--------|------|---------|------|
| v1 mobile ↔ v2 desktop | Full communication | Partial (some features degraded) | Broken |
| Unknown field added | Ignored silently | Logged warning | Parse error |
| Unknown message type | Graceful rejection | Silent drop | Connection closed |
| Version negotiation | Highest common chosen | Falls back to v1 only | No common version |
| Deprecated command | Warning returned | Silent success | Error returned |

## Status

Planned. Not started until EP-001 (transport, pairing, message format) completes.
