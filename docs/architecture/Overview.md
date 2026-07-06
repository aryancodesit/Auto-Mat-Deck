# Architecture Overview

> High-level system architecture for Auto-Mat-Deck.

## System Context

Auto-Mat-Deck is a two-application platform connected via a shared communication protocol:

```
┌──────────────────┐         ┌──────────────────┐
│                  │         │                  │
│   Mobile App     │◄───────►│  Desktop Agent   │
│   (Android)      │ Protocol│  (Execution)     │
│                  │         │                  │
└──────────────────┘         └──────────────────┘
```

- **Mobile App** — Primary user interface. Profiles, deck editing, trigger configuration, dashboards, device discovery and pairing.
- **Desktop Agent** — Execution engine. Runs actions, executes workflows, manages plugins, monitors context, and communicates with the mobile app. No operational UI after onboarding. May expose a minimal **bootstrap interface** (system tray icon, QR dialog) for initial pairing and trust establishment only.
- **Protocol** — The shared wire format and message definitions that both applications implement.

## Bootstrap Interface

The desktop agent has **no operational UI**. After pairing, all interaction happens from the mobile app. However, secure consumer-friendly pairing requires a visual channel (QR code). Therefore:

- **Pre-pairing**: Desktop may show a system tray icon and a QR-code dialog for onboarding.
- **Post-pairing**: All configuration, monitoring, and control happens from the mobile app. The desktop window closes and never reappears.
- **Re-pairing**: If pairing is lost, the bootstrap interface re-activates.

This is the only UI the desktop agent ever presents.

## Design Philosophy

- **Local-first** — No cloud dependency. All data lives on the user's devices.
- **Mobile-primary** — The Android app is the primary interface; the desktop agent is configured from mobile.
- **Protocol-driven** — The communication contract is defined before either application is implemented.
- **Plugin architecture** — Desktop actions are plugin-based. The core agent is thin.

## Key Architectural Decisions

| Decision | Rationale |
|----------|-----------|
| Monorepo | Single source of truth for protocol, shared models, and docs |
| Protocol-first | Mobile and desktop can be built in parallel against a shared contract |
| Headless with bootstrap UI | Secure pairing requires a visual channel; no operational UI after onboarding |
| Plugin-based actions | Extensible without modifying the agent core |

## Validation Approach

Architecture decisions are validated experimentally before freezing. See `spikes/` for disposable prototypes that test critical assumptions (discovery, pairing, transport, encryption) on real devices.
