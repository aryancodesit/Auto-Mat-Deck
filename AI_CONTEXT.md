# AI_CONTEXT.md

> Every AI tool reads this file first to understand the project before generating code or suggestions.

---

## Project Vision

Auto-Mat-Deck is a **local-first**, **mobile-primary** platform for creating, managing, and executing automated workflows. Users configure trigger-action rules on their Android phone and a companion desktop agent executes them.

There is no cloud. No server. No SaaS. Everything runs on the user's own devices.

---

## Architecture Rules

1. **Two applications, one protocol.** The mobile app and desktop agent are independent codebases that communicate exclusively through the shared protocol package. They never import each other's code.

2. **Protocol is the contract.** `packages/protocol/` defines every message, every field, every error code. Mobile and desktop implement the same protocol. If it isn't in the protocol, it doesn't exist.

3. **Desktop agent has no operational UI.** After onboarding, the desktop runs headless — no windows, no settings, no dashboards. It may expose a minimal **bootstrap interface** (system tray icon, QR dialog) for initial pairing only. All user interaction after pairing happens from the mobile app.

4. **Mobile is the primary interface.** All configuration — profiles, decks, triggers, pairing — happens on the Android app. The desktop agent is configured by the mobile app over the protocol.

5. **Plugins extend the desktop.** The desktop agent's action library is plugin-based. The core agent is thin and stable. New capabilities come through plugins, not through modifying the agent.

6. **Mobile owns configuration; desktop owns execution.** Mobile is the source of truth for all user-defined data. Desktop is the source of truth for execution state and context. See [Synchronization Model](./docs/architecture/Synchronization.md).

---

## Folder Rules

```
AUTO_MAT_DECK/
├── apps/                        # Platform applications
│   ├── mobile/                  #   Android app (Jetpack Compose, Kotlin)
│   └── desktop/                 #   Desktop execution agent (bootstrap UI only)
├── packages/                    # Shared packages
│   ├── protocol/                #   Communication protocol (message defs, IDs, codes)
│   └── shared/                  #   Shared resources (icons, schemas, templates, docs)
├── spikes/                      # Disposable validation prototypes
│   └── ep-001-discovery-pairing/
├── docs/                        # Documentation
│   ├── architecture/            #   System design documents
│   ├── releases/                #   Version-specific release notes
│   ├── ui/                      #   Wireframes, mockups, user flows
│   └── decisions/               #   Architectural Decision Records (ADRs)
├── tools/                       # Developer tooling
│   ├── scripts/                 #   Build, test, and utility scripts
│   └── templates/               #   File templates for scaffolding
└── .github/
    └── workflows/               #   CI/CD pipeline definitions
```

### Folder Discipline

- `apps/` contains only deployable applications. Each app has its own build system, dependencies, and CI job.
- `packages/` contains only shared definitions and resources. No application code. No executable entry points.
  - `packages/protocol/` — Schema and message definitions. This is the contract.
  - `packages/shared/` — Shared *resources*: icons, default profile templates, sample workflows, JSON schemas, localization files, validation rules, documentation fragments. **No runtime code.**
- `spikes/` contains disposable validation prototypes. **Never merged into apps/ or packages/.** Deleted after findings are captured in ADRs or architecture docs.
- `docs/` contains only documentation. No code, no data files, no config.
- `tools/` contains only developer-facing scripts. Never shipped to end users.
- Root directory contains only: `README.md`, `AI_CONTEXT.md`, `LICENSE`, `CHANGELOG.md`, `.gitignore`, `Taskfile.yml`.

---

## Naming Conventions

| Category | Convention | Example |
|----------|-----------|---------|
| Repo name | PascalCase | `Auto-Mat-Deck` (GitHub), `AUTO_MAT_DECK` (filesystem) |
| Directories | kebab-case | `apps/mobile/`, `packages/protocol/` |
| Source files | PascalCase or camelCase (per language convention) | `ProfileManager.kt`, `actionRunner.ts` |
| Protocol messages | PascalCase | `ExecuteAction`, `PairRequest` |
| Protocol fields | camelCase | `deviceName`, `protocolVersion` |
| Git branches | kebab-case with type prefix | `feature/v0.1-foundation` |
| Git tags | Semantic version | `v0.1.0`, `v0.2.0`, `v1.0.0` |

---

## Coding Standards

- **Mobile (`apps/mobile/`):** Kotlin, Jetpack Compose, standard Android conventions. Material 3 theming.
- **Desktop (`apps/desktop/`):** TBD — language chosen after protocol is finalized. Candidates: Rust, Go, C# (.NET).
- **Protocol (`packages/protocol/`):** TBD — format chosen after transport spike validates candidates.
- **Shared (`packages/shared/`):** Resources only — no runtime code in any language.
- **Spikes (`spikes/`):** Any language, any framework, any dependency. No constraints. Throwaway code.

### General Principles

- Prefer clarity over cleverness.
- Favor immutability. Data classes/records for all DTOs.
- Explicit error handling. No silent failures. No unchecked exceptions across component boundaries.
- Document public APIs with doc comments. Keep them brief and factual.

---

## Forbidden Dependencies

| Dependency | Reason |
|------------|--------|
| Cloud SDKs (Firebase, AWS, Azure) | Local-first philosophy. No cloud. |
| Remote analytics/crash reporting | No data leaves the user's network. |
| Proprietary/closed-source libraries | Must be buildable by anyone. |
| Python in `apps/` or `packages/` | Python is for `tools/scripts/` only, not for shipped product code. |

---

## Testing Philosophy

- **Protocol tests** validate that messages serialize/deserialize correctly and that version negotiation works. These run on every commit.
- **Unit tests** validate business logic in isolation. Each app has its own unit tests.
- **Integration tests** validate mobile ↔ desktop communication over a real transport. These run nightly.
- **No UI tests in CI** (initially). Android UI tests run locally or on demand.
- Tests live alongside the code they test (`__tests__/` or language-specific convention).

---

## Version Rules

- Each component is versioned independently: **Mobile**, **Desktop**, **Protocol**.
- All three use Semantic Versioning (`MAJOR.MINOR.PATCH`).
- The root `CHANGELOG.md` tracks all releases.
- Release-specific notes go in `docs/releases/vX.Y.Z.md`.
- Git tags are `v`-prefixed: `v0.1.0`, `v0.2.0`, `v1.0.0` — tagged on `main`.
- During discovery/pairing, the desktop advertises all protocol versions it supports; mobile selects the highest mutually supported version.

---

## Branch Strategy

```
main ─── feature/v0.1-foundation ─── feature/v0.2-agent ─── ...
```

- `main` — Always releasable. Production-ready. CI must pass on every commit.
- `feature/*` — Short-lived branches for each version milestone. Merged to `main` via PR.
- Tags — Created on `main` after each merge. No `develop` or `release/*` branches.

---

## Release Workflow

1. Feature branch is completed and merged to `main` via PR.
2. Final testing and documentation updates happen on `main`.
3. A tag is created: `vX.Y.Z`.
4. Release notes are published.
5. Repeat for the next milestone.

---

## Milestone Plan

| Version | Focus |
|---------|-------|
| v0.1 | Foundation — repo, architecture docs, domain model, CI, build orchestration |
| v0.1.5 | **Validation spikes** — EP-001 (discovery, pairing, transport, encryption), EP-002 (Android lifecycle), EP-003 (Windows lifecycle), EP-005 (network resilience) |
| v0.2 | Desktop agent skeleton |
| v0.3 | Android application skeleton |
| v0.4 | Communication layer (mobile ↔ desktop) — includes EP-007 (protocol evolution validation) |
| v0.5 | Profiles |
| v0.6 | Actions — includes EP-006 (action execution validation) |
| v0.7 | Macros |
| v0.8 | Triggers |
| v0.9 | Workflows — includes EP-004 (configuration ownership validation) |
| v1.0 | Production release |

---

## Architecture Freeze

**As of 2026-07-06, the v0.1 architecture is frozen.** No new ADRs, no new architecture documents, no new governance rules will be accepted unless a spike produces reproducible evidence that contradicts an existing architectural assumption.

### Architecture Change Policy

Architecture changes are only permitted when **supported by reproducible evidence collected during an approved spike**. All architectural changes must:

1. Reference the originating spike (e.g., EP-001, EP-003).
2. Update or supersede the affected ADR.
3. Be documented before the spike code is deleted.

Evidence from any spike (EP-001 through EP-007) can legitimately reopen an architectural decision. This is not limited to EP-001.

### Freeze Rules

- Only bug fixes to documentation.
- No new spikes beyond the seven defined (EP-001 through EP-007).
- No new ADRs without spike evidence.
- The next deliverable is **code**: EP-001 implementation.

## AI Instructions

When generating code for this project:

1. Read this file first. Always.
2. Do not introduce cloud dependencies or remote services.
3. Do not suggest Python for shipped application code. Python belongs in `tools/scripts/` only.
4. Do not assume a framework or language has been chosen unless it is specified in an ADR or `docs/architecture/`.
5. When in doubt, ask. Do not make unilateral decisions about architecture, dependencies, or languages.
6. Do not suggest `packages/sdk` — it was removed by ADR and will not be reintroduced until v1.x.
7. Do not merge spike code into `apps/` or `packages/`. Spikes are disposable.
