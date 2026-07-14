# ADR-002: Desktop initiates pairing

**Status:** Accepted
**Date:** 2026-07-10
**Applies To:** v0.2 (OTP pairing)
**Supersedes:** —
**Superseded By:** —
**Engineering Phase:** EP-001, EP-004
**Release:** v0.2

## Current baseline (v0.2)

The v0.2 baseline uses **tray-approval pairing** as documented in
`protocol.md` and implemented in `agent.rs`:

1. Mobile sends `identify` → Desktop.
2. Desktop responds `trusted` (already paired) or `untrusted` (unknown).
3. If untrusted, Mobile sends `pair_request`.
4. Desktop prompts the user via tray notification or GUI to approve or
   decline.
5. Desktop responds `pair_accepted` or `pair_rejected`.

This ADR describes a future enhancement to replace step 4 with an
OTP-based flow. The decision text is preserved here for architectural
history and will be activated when OTP pairing is implemented.

## Context

Pairing requires a human-verifiable secret (OTP) to establish trust.
Either Desktop or Mobile could generate and display the OTP.

## Decision

**Desktop initiates pairing** by generating the OTP and sending it to
Mobile. Mobile displays the OTP; the user reads it from their phone and
enters it into the Desktop GUI.

## Consequences

- **Positive:** Desktop generates the OTP, verifies it, and marks the device
  as paired in a single transaction. Mobile only needs to display a 4-digit
  code — minimal trust surface.
- **Negative:** The user must look at their phone and then type on their
  keyboard. Slightly slower than the reverse direction, but safer.

## Rationale

- Mobile is more likely to be stolen or lost than Desktop.
- If Mobile generated the OTP, a compromised phone could pair itself
  without the user's knowledge.
- Desktop is in a physically controlled environment (home/office).
- Entering a short code on a hardware keyboard is faster and less error-prone
  than typing on a phone.

## Migration path

When OTP pairing is implemented:

1. Add new protocol messages (`pair_challenge`, `pair_verify`).
2. Update `protocol.md` with new message formats and sequence diagram.
3. Update this ADR: change Status to "Accepted", add the release number.
4. Update `desktop.md` pairing section if needed.

The existing tray-approval flow will remain as a fallback mechanism.
