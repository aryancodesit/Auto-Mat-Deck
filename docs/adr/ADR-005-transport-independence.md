# ADR-005: Transport Independence

**Status:** Accepted
**Date:** 2026-07-10

## Context

The current Desktop implementation embeds WebSocket handling directly in
`agent.rs`. Business logic (pairing, action execution, device identity) and
transport concerns (connection lifecycle, message framing, streaming) are
tightly coupled.

Future transports — USB, Bluetooth, local named pipes, Unix sockets, or
TCP — would require rewriting the connection handler for each.

## Decision

Business logic **must not depend on WebSocket**. Communication occurs
through a **transport abstraction**:

```
Mobile
  │
  ▼
Transport Adapter   (WebSocket, USB, Bluetooth, …)
  │
  ▼
Protocol Layer      (message serialization, framing, routing)
  │
  ▼
Execution Engine    (pairing, actions, context, config)
```

- Each transport implements a common `Transport` trait (connect, disconnect,
  send, receive, on_event).
- The protocol layer is transport-agnostic — it only sees framed messages.
- Adding a new transport requires implementing the trait, not modifying
  business logic.

## Consequences

- **Positive:** New transports can be added without touching pairing,
  actions, or device management. Testing is easier — mock transports can
  simulate the network without real sockets.
- **Negative:** Initial extraction cost. The existing inline WebSocket code
  must be refactored into the trait + protocol layer.
- **Neutral:** The abstraction adds a small indirection, but the cost is
  negligible compared to the benefit of transport flexibility.

## Rationale

- The protocol is already JSON-based and transport-agnostic in theory.
  Making it transport-agnostic in code prevents coupling from creeping in.
- History shows that desktop tools often need multiple transports
  (USB tethering when WiFi is unavailable, Bluetooth for low-power
  scenarios, local pipes for loopback).
- The pattern is well-established (see: `net/http` in Go, `tower` in Rust).
