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

## Dependency Graph

```
EP-001  (Discovery, Pairing, Transport, Encryption)
  │
  ├───────────────┐
  │               │
  ▼               ▼
EP-002         EP-003
Android        Windows
  │               │
  └───────┐   ┌───┘
          ▼   ▼
        EP-005
    Network Resilience

EP-004  (Configuration Ownership — depends on EP-001 + EP-003)
EP-006  (Action Execution — depends on EP-003)
EP-007  (Protocol Evolution — depends on EP-001)
```

## Evidence Template

Every spike must produce and document these artifacts before deletion:

### Measurements
| Metric | Measured Value | Conditions | Threshold |
|--------|----------------|------------|-----------|
| Discovery time | | | PASS <5s / WARN 5-10s / FAIL >10s |
| Handshake time | | | PASS <2s / WARN 2-5s / FAIL >5s |
| Round-trip latency | | | PASS <500ms / WARN 500-1000ms / FAIL >1000ms |
| Reconnection time | | | PASS <10s / WARN 10-30s / FAIL >30s |

### Compatibility Matrix
| OS / Network Type | Result | Notes |
|-------------------|--------|-------|
| Windows 11, same Wi-Fi | | |
| Windows 10, same Wi-Fi | | |
| Wired desktop + Wi-Fi mobile | | |
| Public Wi-Fi / AP isolation | | |
| Enterprise Wi-Fi | | |

### Known Limitations
- [Limitation discovered during testing]

### Recommendations
- [What should the architecture adopt?]

### ADR Updates
- [Which ADRs were confirmed, which were changed?]

## Index

| Spike | Question | Depends On | Status |
|-------|----------|------------|--------|
| [ep-001](./ep-001-discovery-pairing/) | Discovery, pairing, transport, encryption | — | Planned |
| [ep-002](./ep-002-android-lifecycle/) | Android background execution, Doze, foreground service | EP-001 | Planned |
| [ep-003](./ep-003-windows-lifecycle/) | Windows service, firewall, startup, sleep/wake | EP-001 | Planned |
| [ep-004](./ep-004-configuration-ownership/) | Configuration ownership and multi-device authority | EP-001, EP-003 | Planned |
| [ep-005](./ep-005-network-resilience/) | Network resilience — DHCP, hostname, AP isolation, captive portals | EP-001 | Planned |
| [ep-006](./ep-006-action-execution/) | Action execution — process isolation, timeout, cancellation | EP-003 | Planned |
| [ep-007](./ep-007-protocol-evolution/) | Protocol evolution — version negotiation, backward compatibility | EP-001 | Planned |
