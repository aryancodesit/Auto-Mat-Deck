# Overview

AutoMatDeck is a **simple, personal, local-first** remote desktop tool.
It lets you trigger desktop actions from your phone — no cloud, no accounts,
just your local network.

## Architecture

Two-component system:

| Component | Role | Platform |
|-----------|------|----------|
| **Desktop** (AutoMatDeck Desktop) | Owns system state, configuration, execution, automation, and context | Windows (native exe) |
| **Mobile** (AutoMatDeck Remote) | Primary user interface for remote operation | Android (native app) |

## Communication

1. **mDNS** — Desktop advertises its presence on the LAN; Mobile discovers it.
2. **WebSocket** — Once discovered, Mobile connects to Desktop over WS for all
   subsequent messaging.
3. **OTP Pairing** — One-time PIN displayed on Mobile, entered on Desktop to
   establish trust.

## Repository layout

```
AUTO-MAT-DECK/
├── apps/           # Production application code
│   ├── desktop/    # Rust (eframe/tokio) — Windows desktop daemon
│   └── mobile/     # Kotlin (Jetpack Compose) — Android remote (EP-005+)
├── shared/         # Cross-component definitions
│   └── protocol/   # Wire protocol schema & versioning
├── docs/           # Architecture, decisions, release notes
│   ├── architecture/
│   ├── adr/
│   ├── ep/
│   └── releases/
├── scripts/        # Developer utilities
│   ├── build/
│   ├── adb/
│   └── dev/
└── spikes/         # Experimental / throwaway prototypes
```

## Design principles

- **No cloud dependency** — everything runs on your LAN.
- **Desktop authority** — Desktop owns system state and execution. Mobile is the primary user interface for remote operation.
- **Secure by default** — Unknown devices cannot execute actions. Pairing is
  required before trust is established. Native OS APIs are preferred over
  shell interpreters. Security decisions favor explicit user approval over
  implicit automation.
- **Protocol-driven** — Desktop and Mobile only interact through the shared
  wire protocol.
- **Spike-before-product** — new ideas are validated in `spikes/` before
  becoming production code.
