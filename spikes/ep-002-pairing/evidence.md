# EP-002 Evidence

## Build

| Component | Result |
|-----------|--------|
| Desktop agent (`cargo build --release`) | ✅ PASS |
| Android app (`gradlew assembleDebug`) | ✅ PASS |

## Pairing Flow

| Test | Result | Notes |
|------|--------|-------|
| Pair request → accepted | ⏳ Pending | Needs hardware test |
| Pair request → rejected | ⏳ Pending | Needs hardware test |
| Trusted device auto-reconnect | ⏳ Pending | Needs hardware test |
| Unknown device rejected | ⏳ Pending | Needs hardware test |
| Persist after desktop restart | ⏳ Pending | Needs hardware test |
| Persist after phone restart | ⏳ Pending | Needs hardware test |

## Artifacts

- `%APPDATA%/AutoMatDeck/trusted_devices.json`
- Logcat output
