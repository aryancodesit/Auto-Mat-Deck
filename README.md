# Auto-Mat-Deck

**Automated Material Deck** — A local-first platform for creating, managing, and executing automated workflows across mobile and desktop environments.

**Status:** v0.6.0 released — Workflow engine certified, desktop frozen, Android integration complete.

## Platform

Auto-Mat-Deck is a multi-application platform:

| Application | Role |
|-------------|------|
| **Mobile** (`apps/mobile/`) | Android app — UI for profiles, decks, triggers, pairing, and dashboard |
| **Desktop** (`apps/desktop/`) | Execution engine — runs actions, workflows, plugins, and device coordination |

### Architecture

```
Android
    │
    ▼
WebSocket Transport (JSON, local-first, no cloud)
    │
    ▼
agent.rs (coordinator)
    │
    ▼
ExecutionTarget
    │
    ▼
execute_target()
    ├───────────────┐
    ▼               ▼
execute_action()  execute_workflow()
```

## Repository Structure

```
AUTO_MAT_DECK/
├── apps/           # Platform applications (mobile + desktop)
├── packages/       # Shared protocol and resources
├── docs/           # Architecture, decisions, UI docs, release notes
├── spikes/         # Validation spikes (EP-001 discovery/pairing)
├── tools/          # Developer scripts and templates
└── .github/        # CI/CD workflows
```

## Development

```bash
# Desktop (Rust)
cd apps/desktop
cargo test
cargo clippy

# Android (Kotlin)
cd spikes/ep-001-discovery-pairing/mobile
gradlew test
```

See [AI_CONTEXT.md](./AI_CONTEXT.md) for the complete project guide — architecture rules, naming conventions, versioning, and standards every contributor follows.

## Release History

| Version | Tag | Status |
|---------|-----|--------|
| v0.6.0 | `v0.6.0` | RELEASED — Workflow engine |
| v0.5.0 | `v0.5.0` | RELEASED — Control surface + execution |
| v0.1 | `v0.1-ep003-certified` | RELEASED — Discovery, trust, 5 actions |

## License

[MIT](./LICENSE)
