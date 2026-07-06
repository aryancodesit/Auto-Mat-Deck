# EP-002 Evidence

## Build

| Component | Result |
|-----------|--------|
| Desktop agent (`cargo build --release`) | ✅ PASS |
| Android app (`gradlew assembleDebug`) | ✅ PASS |

## Hardware

| Device | Detail |
|--------|--------|
| Desktop | 192.168.29.59 |
| Phone | realme RMX3392, Android 14 — 192.168.29.56 |

## Pairing Flow

| Test | Result | Notes |
|------|--------|-------|
| Discovery | ✅ PASS | Desktop found via mDNS |
| WebSocket connect | ✅ PASS | `New connection from 192.168.29.56:33714` |
| Device identification | ✅ PASS | UUID `a5d575ed-46f6-40ee-878f-e40057642d27` sent |
| Pair request → accepted | ✅ PASS | Console `[y/N]: y` → `Paired with device: Android-RMX3392` |
| Pair request → rejected | ⏳ Pending | Needs hardware test |
| Ping/pong after pair | ✅ PASS | 11ms RTT |
| Trusted device auto-reconnect | ✅ PASS | Phone auto-connected without re-pairing |
| Unknown device rejected | ✅ PASS | Re-pair required after deleting trusted_devices.json |
| Persist after desktop restart | ✅ PASS | Trust survived agent restart |
| Persist after phone restart | ✅ PASS | Trust survived app restart |

## Artifacts

- `C:\Users\aryan\AppData\Roaming\AutoMatDeck\trusted_devices.json`
