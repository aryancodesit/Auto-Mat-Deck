# spikes/

> Disposable validation prototypes. Evidence preserved. Code deleted.

## Purpose

Spikes answer questions that architecture docs cannot. Before freezing the protocol, before choosing a transport, before committing to an encryption scheme — spike it on real devices.

Every spike answers **one architectural question**. If a spike begins answering multiple unrelated questions, split it into another spike. This rule prevents validation work from turning into prototype product development.

Each spike is a self-contained experiment. The code is throwaway. The **evidence is not**.

## Principles

- Spikes are **never** merged into `apps/` or `packages/`.
- Spike code is **deleted** after findings are captured in ADRs or architecture docs.
- Spikes can use any language, any library, any framework. No constraints.
- A failed spike is a success — it saved us from a wrong decision.
- **One spike, one question.** If a spike grows beyond its original question, split it.

## Exit Criteria

A spike ends only when exactly one of these outcomes is reached:

1. **Architecture validated** — all success thresholds met or within WARNING range. No FAIL thresholds triggered.
2. **Architecture rejected** — a FAIL threshold proves the current design cannot work. Architecture must change.
3. **Architecture changed** — evidence shows a different approach is superior. New approach is documented in an ADR before the spike is deleted.

A spike **never** ends with "needs more investigation." If more questions remain, create a new spike.

## Success Thresholds

Every metric in every spike must carry a three-tier threshold:

| Grade | Meaning |
|-------|---------|
| **PASS** | Meets target — architecture validated |
| **WARNING** | Acceptable but needs attention before production |
| **FAIL** | Below minimum — architecture must change |

## Version Roadmap

```
v0.1 — Core Communication          ✅ Certified
├── EP-001  Discovery & Transport   ✅ Done
├── EP-002  Pairing & Trust         ✅ Done
├── EP-002.5  Desktop Packaging     ✅ Done
└── EP-003  Remote Actions          ✅ Done
        ↓
  Release v0.1 — All foundations complete

v0.2 — Native Execution + Mobile Command Deck
├── TD-001  Native Execution Layer    📋 Planned (remove cmd/powershell from action path)
└── EP-004  Mobile Command Deck (pages, buttons, icons, profiles, app launch)
v0.3 — Context Awareness (work, gaming, coding, battery profiles)
v0.4 — Automation (triggers, macros, pipelines)
v0.5 — Plugin Ecosystem
v0.6 — Polish
v0.7 — Public Beta
v1.0 — Production Stable
```

## Index

| Spike | Version | Status |
|-------|---------|--------|
| [ep-001](./ep-001-discovery-pairing/) | v0.1 — Discovery & Transport | ✅ Certified |
| [ep-002](./ep-002-pairing/) | v0.1 — Pairing & Trust | ✅ Certified |
| EP-002.5 | v0.1 — Desktop Packaging | ✅ Done (embedded in ep-001 code) |
| [ep-003](./ep-003-remote-actions/) | v0.1 — Remote Actions | ✅ Certified |
| [td-001](./td-001-native-execution-layer/) | v0.2 — Native Execution Layer | 📋 Planned |
