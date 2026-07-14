# Protocol Version

**Current version:** v0.1-draft

## Version scheme

```
v0   →  v0.1  →  v0.2  →  v1 (stable)
draft    draft    draft    semver
```

Protocol will follow [semantic versioning](https://semver.org/) only after
reaching a stable wire format at v1.

## Compatibility

| Version | Status | Desktop | Mobile |
|---------|--------|---------|--------|
| v0.1-draft | Active (may change) | v0.x | v0.x |

## Changelog

### v0.1-draft (2026-07-10)

- Initial draft protocol.
- WebSocket transport, JSON messages.
- Message types: `identify`, `trusted`, `untrusted`, `pair_request`,
  `pair_accepted`, `pair_rejected`, `action`, `action_result`, `ping`, `pong`,
  `error`.
