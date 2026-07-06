# EP-002 — Pairing & Trust

> **Goal:** Transform discovery into trusted communication.
> **Part of:** v0.1 — Core Communication

---

## Objective

Convert raw WebSocket connection into a trusted channel. After EP-002, the phone pairs once and is recognized on every subsequent connection — no re-pairing.

---

## Scope

| Area | Included |
|------|----------|
| Pair request flow | ✅ Phone requests, desktop approves via console |
| Trusted device persistence | ✅ `trusted_devices.json` on desktop |
| Device identity | ✅ Phone generates UUID once, persists locally |
| Auto-reconnect | ✅ Trusted devices skip pair flow on reconnect |
| Unknown device rejection | ✅ Untrusted connections rejected |
| Desktop approval | ✅ Console `[y/n]` prompt |
| Pair status display | ✅ Phone shows "Paired ✓" or "Not paired" |
| Unpair management | ❌ v0.2 |
| Pair list UI | ❌ v0.2 |
| Encryption | ❌ Future |
| Command execution | ❌ EP-003 |

---

## Implementation Checklist

### Desktop (Rust)

- [ ] **Pair request handler** — Handle `"pair_request"` with `device_id` + `device_name`
- [ ] **Console approval** — Print incoming request, wait for `y/n`, respond to phone
- [ ] **Trusted device store** — `trusted_devices.json` with fields: `device_id`, `device_name`, `last_seen`, `paired_at`
- [ ] **Connection gate** — On new WebSocket `"identify"`:
  - Trusted → allow
  - Unknown → respond `"untrusted"` and close
- [ ] **Auto-reconnect** — Trusted devices bypass pair flow entirely
- [ ] **Logging** — Log paired, rejected, untrusted connection events

### Mobile (Android/Kotlin)

- [ ] **Device identity** — Generate UUID on first launch, persist in SharedPreferences
- [ ] **Pair button** — After discovery, tap to send `"pair_request"`
- [ ] **Pair response handling** — Show "Paired ✓" or "Rejected" in status
- [ ] **Identify on connect** — Send `"identify"` with `device_id` on WebSocket open
- [ ] **Auto-reconnect** — On app start, scan for known device, auto-connect if trusted
- [ ] **Handle rejection** — Show "Device not trusted" if desktop rejects

### Protocol (JSON messages)

- [ ] `{"type": "identify", "device_id": "..."}` — Sent on connect
- [ ] `{"type": "pair_request", "device_id": "...", "device_name": "..."}` — Request pairing
- [ ] `{"type": "pair_accepted", "device_id": "..."}` — Desktop accepted
- [ ] `{"type": "pair_rejected", "device_id": "...", "reason": "..."}` — Desktop rejected
- [ ] `{"type": "untrusted", "message": "..."}` — Unknown device rejected

---

## Testing Checklist

| Test | Expected | How |
|------|----------|-----|
| Pair accepted | Phone shows "Paired ✓" | Accept on desktop console |
| Pair rejected | Phone shows "Rejected" | Decline on desktop console |
| Reconnect trusted | Auto-connected, no pair prompt | Kill + reopen app |
| Unknown device rejected | Phone shows "Not trusted" | Connect from different client |
| Persist after desktop restart | Trusted device still recognized | Restart desktop agent |
| Persist after phone restart | Device identity still sent | Restart phone app |

---

## Evidence Checklist

- [ ] Logcat: pair request → accepted
- [ ] Console: approval prompt shown
- [ ] `trusted_devices.json` file with valid content
- [ ] Logcat: unknown device rejected
- [ ] Logcat: trusted auto-reconnect (no pair prompt)

---

## Git Milestone

```
v0.1
├── EP-001  Discovery & Transport   ✅ (tag: v0.1-ep001-certified)
├── EP-002  Pairing & Trust         🔜 (tag: v0.1-ep002-certified)
└── EP-003  Command Execution
        ↓
  Release v0.1
```

---

## Architecture Note

Extends existing code only — no new abstractions:
- `main.rs` — add message handlers for identify, pair_request
- `MainActivity.kt` — add identity persistence, pair button, status display
- `DiscoveryProvider` / `AdvertisementProvider` — unchanged

Protocol is plain JSON over existing WebSocket. No transport changes.
