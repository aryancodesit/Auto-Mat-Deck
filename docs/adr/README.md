# Architecture Decision Records

## Purpose

An Architecture Decision Record (ADR) captures a significant architectural
decision, including the context that prompted it, the chosen approach, and
the consequences. ADRs are permanent — they are never rewritten, only
superseded by newer ADRs.

## When to create an ADR

Create an ADR when a decision:

- Affects the system's structure, boundaries, or communication patterns.
- Has long-lived consequences (years, not sprints).
- Would be expensive to reverse.
- Needs to be explained to future contributors.

Do **not** create an ADR for:

- Implementation details (library versions, variable names).
- Operational or procedural choices (CI cadence, ticket labels).
- Protocol timing or default values (heartbeat intervals, timeouts).

Those belong in protocol documentation or implementation comments.

## ADR lifecycle

```
                  ┌──────────┐
                  │ Proposed │
                  └────┬─────┘
                       │ Approved
                       ▼
                  ┌──────────┐
                  │ Accepted │
                  └────┬─────┘
                       │ Released
                       ▼
                 ┌────────────┐
                 │ Implemented│
                 └─────┬──────┘
                       │
              ┌────────┴────────┐
              │                 │
              ▼                 ▼
       ┌────────────┐   ┌─────────────┐
       │ Deprecated │   │ Superseded  │
       └────────────┘   └─────────────┘
```

| Status | Meaning |
|--------|---------|
| **Proposed** | Under discussion, not yet approved. |
| **Accepted** | Approved as the architectural direction, but not yet released. |
| **Implemented** | Reflected in the released software. |
| **Deprecated** | No longer recommended. A newer ADR explains why. |
| **Superseded** | Replaced by another ADR. The original text is preserved for history. |

## Naming convention

Files follow the pattern:

```
ADR-NNN-title-with-dashes.md
```

- `NNN` is a zero-padded, sequential number (001, 002, …).
- The title is a short, hyphenated slug derived from the decision.

Examples:

```
ADR-001-desktop-owns-configuration.md
ADR-002-desktop-initiates-pairing.md
```

## Metadata fields

Every ADR begins with a YAML-style metadata block:

```
# ADR-NNN: Title

**Status:** <lifecycle status>
**Date:** YYYY-MM-DD
**Applies To:** <scope, e.g. v0.x>
**Supersedes:** <ADR-NNN or —>
**Superseded By:** <ADR-NNN or —>
**Engineering Phase:** <EP-NNN or —>
**Release:** <v0.x or —>
```

| Field | Required | Description |
|-------|----------|-------------|
| `Status` | Yes | Current lifecycle phase. |
| `Date` | Yes | Original creation date (not updated for metadata changes). |
| `Applies To` | Yes | Which version or scope the decision covers. |
| `Supersedes` | Yes | ADR this one replaces, or `—` if none. |
| `Superseded By` | Yes | ADR that replaced this one, or `—` if none. |
| `Engineering Phase` | Recommended | EP that produced this decision. |
| `Release` | Recommended | Release where the decision was first shipped. |

## Current ADRs

| # | Title | Status | Release |
|---|-------|--------|---------|
| 001 | Desktop owns configuration | Implemented | v0.2 |
| 002 | Desktop initiates pairing | Proposed | — |
| 003 | Native execution over shell | Implemented | v0.2 |
| 004 | Bind WebSocket before advertising | Implemented | v0.2 |
| 005 | Transport independence | Accepted | — |
| 006 | Context is Advisory | Accepted | — |
| 007 | Persistent entities use immutable IDs | Implemented | v0.2 |
