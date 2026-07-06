# Security Model

> Trust establishment, key storage, session management, and threat mitigation.

## Principles

1. **No cloud** — all security is local. No remote attestation, no third-party identity providers.
2. **User-mediated trust** — pairing always requires explicit user confirmation on both sides.
3. **Defense in depth** — multiple layers: network isolation, authenticated messages, local key storage.
4. **Minimum attack surface** — only the protocol ports are exposed; no unnecessary services.

## Trust Establishment

- See [Pairing & Authentication](./Pairing-Authentication.md) for the pairing flow.
- Trust is established out-of-band (QR scan or code entry).
- The shared secret from pairing is used to derive session keys.

### MITM Verification

The pairing protocol must provide defenses against man-in-the-middle attacks on the local network:

| Protection | Mechanism |
|------------|-----------|
| Device identity verification | Mobile verifies desktop's persistent identity matches the paired record |
| Fingerprint verification | User can compare a displayed fingerprint on both devices before confirming |
| Session binding | Session keys are bound to the specific pairing handshake; replay of old material is rejected |
| Replay protection | Every message carries a session-scoped sequence number; duplicates are rejected |
| Pairing replay | Pairing material from a previous session is rejected after reboot |

These protections are validated experimentally in [spike ep-001](../../spikes/ep-001-discovery-pairing/).

## Key Storage

- Long-term keys are stored in platform keychains:
  - **Android**: Android Keystore (hardware-backed where available)
  - **Desktop**: OS keychain (Windows Credential Manager, macOS Keychain, Linux Secret Service)
- Session keys are held in memory only and discarded on disconnect.

## Session Security

- Every message includes a message authentication code (MAC) derived from the session key.
- Sequence numbers prevent replay attacks.
- Sessions expire after a configurable idle timeout.
- Expired sessions require re-authentication (not re-pairing).

## Threat Model

| Threat | Mitigation |
|--------|-----------|
| Eavesdropping | Encrypted transport (TLS/Noise) |
| Replay attacks | Sequence numbers + session-scoped MAC |
| Man-in-the-middle | Out-of-band pairing confirmation + fingerprint verification |
| Unauthorized pairing | User must confirm on both devices |
| Device theft | Local key storage + optional app lock |
| Rogue desktop on LAN | User must explicitly pair; mobile shows desktop identity |
| Message tampering | Authenticated messages (MAC verified on receipt) |
| Pairing replay after reboot | Fresh handshake; old pairing material rejected |

## Out of Scope (v0.1)

- Perfect forward secrecy (can be added later)
- Certificate-based identity (overkill for local-first)
- Remote revocation (no cloud to coordinate through)
- Audit logging (local logs only, no centralized collection)
