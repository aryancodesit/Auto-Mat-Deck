# Communication Protocol

> Defines how the mobile app and desktop agent communicate.

## Transport

**Not yet selected.** See [ADR-006: Transport Evaluation](../decisions/ADR-006-transport-evaluation.md) for evaluation criteria and candidate comparison. Transport will be chosen after spike ep-001 validates discovery, pairing, and encryption on real devices.

Current candidates:

- WebSocket (strong candidate — native on Android and Windows, full-duplex, supports WSS)
- Raw TCP with Noise encryption (viable, more implementation effort)
- USB/ADB (fallback for wired-only scenarios)

## Message Format

**Deferred.** The validation spike (ep-001) will use **JSON** for simplicity — it is human-readable, easy to inspect, and requires no schema compilation. Production message format will be evaluated after:

- Profiling JSON performance under realistic message volumes.
- Understanding schema evolution needs (backward compatibility, field deprecation).
- Comparing schema-based formats (Protocol Buffers, FlatBuffers) against JSON.

JSON is **not** a commitment for v1.0. It is a prototyping convenience.

| Format | Use |
|--------|-----|
| JSON | Spike ep-001 only |
| TBD | Production (evaluated after transport is chosen) |

## Message Categories

| Category | Direction | Examples |
|----------|-----------|---------|
| Discovery | Bidirectional | `DeviceAnnounce`, `DeviceQuery` |
| Pairing | Bidirectional | `PairRequest`, `PairConfirm`, `PairReject` |
| Heartbeat | Bidirectional | `Ping`, `Pong` |
| Command | Mobile → Desktop | `ExecuteAction`, `RunMacro`, `TriggerWorkflow` |
| Response | Desktop → Mobile | `ActionResult`, `MacroResult`, `WorkflowStatus` |
| Event | Desktop → Mobile | `ContextChanged`, `TriggerFired`, `PluginEvent` |
| Capability | Bidirectional | `CapabilityQuery`, `CapabilityReport` |

## Version Negotiation

- Both parties advertise supported protocol versions during pairing.
- Communication uses the highest mutually supported version.

## Security

- Pairing requires user confirmation on both devices.
- All messages after pairing are authenticated.
- No secrets transmitted in plaintext.
- Production transport will use encrypted channels (WSS, Noise, or TLS).
