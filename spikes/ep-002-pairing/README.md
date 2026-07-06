# EP-002 — Pairing & Trust

> **Goal:** Transform discovery-only prototype into a trusted paired system.
> **Part of:** v0.1 — Core Communication (EP-001 Discovery + EP-002 Pairing + EP-003 Execution)

---

## Objective

Convert raw mDNS discovery + WebSocket connection into a trusted communication channel.
After EP-002, the phone can discover, pair with, and be recognized by the desktop — without re-pairing on every connection.

---

## Scope

| Area | Included | Out of Scope |
|------|----------|--------------|
| Pairing request flow | ✅ Phone requests, desktop approves | |
| Desktop approval UI | ✅ Minimal (console accept/reject for spike) | |
| Trusted device persistence | ✅ Desktop remembers paired devices | |
| Auto-reconnect | ✅ Trusted devices reconnect without re-pairing | |
| Reject unknown devices | ✅ Unknown connections rejected | |
| Pair/unpair management | ✅ List, remove paired devices | |
| Encryption | | ❌ Deferred |
| Command execution | | ❌ EP-003 |
| Context profiles | | ❌ v0.3+ |
| Plugin system | | ❌ v0.5+ |

---

## Implementation Checklist

### Desktop (Rust)

- [ ] **Pair request handler** — New WebSocket message type `"pair_request"` containing `device_id` and `device_name`
- [ ] **Desktop approval** — Console prompt (Y/N) when a pair request arrives; log + respond to phone
- [ ] **Trusted device store** — Simple JSON file (`trusted_devices.json`) in a known location (`%APPDATA%/AutoMatDeck/` or `./data/`)
  - Fields: `device_id`, `device_name`, `last_seen`, `paired_at`
- [ ] **Pair response** — Respond with `"pair_accepted"` or `"pair_rejected"` message type
- [ ] **Connection gate** — On new WebSocket connection, check if device is trusted:
  - Device sends `"identify"` message with `device_id`
  - If trusted → allow full communication
  - If unknown → respond with `"untrusted"` and close
- [ ] **Auto-reconnect** — When a trusted device reconnects, skip pair flow
- [ ] **Unpair endpoint** — Handle `"unpair"` message to remove device from store
- [ ] **Logging** — Log pair events (paired, rejected, untrusted connection)

### Mobile (Android/Kotlin)

- [ ] **Pair button** — After discovery, show "Pair" instead of auto-connecting
- [ ] **Pair request flow** — Send `"pair_request"` with local `device_id` and `device_name`
- [ ] **Pair response handling** — Show "Paired" or "Rejected" feedback
- [ ] **Trusted device list** — Display paired devices (can be a simple list view)
- [ ] **Device identity** — Generate and persist a local `device_id` (UUID) in SharedPreferences
- [ ] **Auto-reconnect** — On app start, scan for known trusted devices and reconnect automatically
- [ ] **Unpair** — Button to remove a device from the trusted list
- [ ] **Identify on connect** — Send `"identify"` with `device_id` on WebSocket open
- [ ] **Handle rejection** — Show "Device not trusted" if desktop rejects

### Protocol (JSON messages)

- [ ] `{"type": "identify", "device_id": "..."}` — Sent by client on connect
- [ ] `{"type": "pair_request", "device_id": "...", "device_name": "..."}` — Request pairing
- [ ] `{"type": "pair_accepted", "device_id": "..."}` — Desktop accepted
- [ ] `{"type": "pair_rejected", "device_id": "...", "reason": "..."}` — Desktop rejected
- [ ] `{"type": "untrusted", "message": "..."}` — Desktop rejects unknown device
- [ ] `{"type": "unpair", "device_id": "..."}` — Remove pairing
- [ ] `{"type": "unpaired", "device_id": "..."}` — Confirm unpair

---

## Testing Checklist (Lightweight)

| Test | Expected | How |
|------|----------|-----|
| Pair request accepted | Phone shows "Paired" | Accept on desktop console |
| Pair request rejected | Phone shows "Rejected" | Decline on desktop console |
| Trusted device reconnects | Auto-connected, no pair prompt | Kill + reopen app |
| Unknown device rejected | Phone shows "Not trusted" | Connect from a different client |
| Unpair | Device removed from trusted list | Tap unpair, reconnect → should re-pair |
| Paired device persists after desktop restart | Trusted device still recognized | Restart desktop agent |
| Paired device persists after phone restart | Device still in phone's list | Restart phone app |
| Re-pair after unpair | Works like first-time pairing | Unpair → pair again |

---

## Evidence Checklist

- [ ] Logcat: pair request → accepted flow captured
- [ ] Desktop console: pair approval prompt shown
- [ ] `trusted_devices.json` file created with correct content
- [ ] Logcat: unknown device rejected
- [ ] Logcat: trusted device auto-reconnect (no pair prompt)
- [ ] Screenshot or video: paired device in phone UI
- [ ] Summary documented in `evidence.md`

---

## Git Milestone Plan

```
v0.1
├── EP-001  —— Discovery & Transport  ✅ Done (tag: v0.1-ep001-certified)
├── EP-002  —— Pairing & Trust         🔜 This spike
└── EP-003  —— Command Execution
                ↓
         Release v0.1
```

Branch: `spike/ep-002-pairing` (or work on `main` for simple spikes)

Tag upon completion: `v0.1-ep002-certified`

---

## Architecture Note

No new abstractions required. EP-002 extends the existing:
- `main.rs` WebSocket message handler → add pair/unpair/identify handlers
- `MainActivity.kt` → add pair button, trusted device list, identity persistence
- `DiscoveryProvider` / `AdvertisementProvider` → unchanged

The pairing protocol is plain JSON over the existing WebSocket — no protocol changes needed.
