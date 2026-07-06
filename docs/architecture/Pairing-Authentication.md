# Pairing & Authentication

> How devices establish trust and authenticate communication.

## Bootstrap UI — Not Yet Decided

The desktop agent has no operational UI, but pairing requires a visual or interactive channel for secure trust establishment. The bootstrap mechanism is **actively being evaluated** rather than prescribed.

Three onboarding methods will be prototyped and compared in [spike ep-001](../../spikes/ep-001-discovery-pairing/):

| Method | Description |
|--------|-------------|
| **Native window** | Lightweight native window showing a QR code; system tray icon; auto-closes after pairing |
| **Browser page** | Embedded HTTP server on localhost; browser renders QR code |
| **CLI pairing** | Pairing code printed to stdout; user types into mobile app |

The decision will be recorded in an ADR after spike results are analyzed. Until then, the architecture does not commit to any specific bootstrap mechanism.

## Pairing Flow

1. **Discovery** — Mobile discovers desktop on the LAN (see [Discovery](./Discovery.md)).
2. **Initiation** — User selects a desktop from the discovery list and taps "Pair."
3. **Challenge** — Desktop presents pairing material (QR code or code) via the chosen bootstrap method.
4. **Confirmation** — User scans the QR (or enters the code) on mobile. A secure handshake completes.
5. **Key Exchange** — Devices exchange public keys and establish a shared session secret.
6. **Verification** — Both devices store the peer's identity and verify the session.
7. **Completion** — Desktop is now paired. Bootstrap UI closes. Desktop listed in mobile app.

## Trust Model

- Pairing is **explicit** — user must confirm on both devices.
- Once paired, devices remember each other across restarts.
- Pairing is **one-to-one** for each profile, but a desktop can be paired with multiple mobiles.

## Session Management

- After pairing, all communication uses the established session.
- Sessions have a configurable timeout.
- Session renewal uses the stored shared secret (no re-pairing required).
- If a session is compromised, the user can force re-pairing.

## Authentication

- Every message after pairing is authenticated using the session key.
- Message authentication prevents tampering and replay attacks.
- The protocol defines which fields are authenticated vs. plaintext.

## State Diagram

```
Discovered → Pending → Paired → Active
                ↓          ↓
            Rejected    Unpaired → Discovered
                ↓
            Forgotten
```
