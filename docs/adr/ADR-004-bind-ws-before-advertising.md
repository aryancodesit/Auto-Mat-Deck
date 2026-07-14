# ADR-004: Bind WebSocket before advertising via mDNS

**Status:** Accepted
**Date:** 2026-07-10

## Context

When Desktop starts, it needs both a WebSocket server (for Mobile to
connect to) and mDNS advertisement (so Mobile can discover it). The order
matters for correctness.

## Decision

The Desktop **binds the WebSocket server first**, then begins mDNS
advertising. It never advertises an endpoint that is not yet accepting
connections.

## Consequences

- **Positive:** Mobile will never resolve a Desktop that rejects its
  WebSocket handshake. Connection reliability improves.
- **Negative:** A window exists between WS bind and mDNS start where
  Desktop is technically reachable but not discoverable. This is harmless
  (Mobile just sees it slightly later).

## Rationale

- An mDNS record pointing to a non-listening port creates a failure mode
  that is hard to diagnose from Mobile.
- The startup sequence is fast (< 100ms); the window is negligible.
- This matches the robustness principle: be conservative in what you
  advertise.
