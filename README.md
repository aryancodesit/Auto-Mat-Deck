# Auto-Mat-Deck

**Automated Material Deck** — A local-first platform for creating, managing, and executing automated workflows across mobile and desktop environments.

**Status:** Early development (pre-v0.1.0) · EP-001 complete — mDNS discovery, WebSocket transport, and ping/pong validated on real hardware.

## Platform

Auto-Mat-Deck is a multi-application platform:

| Application | Role |
|-------------|------|
| **Mobile** (`apps/mobile/`) | Android app — UI for profiles, decks, triggers, pairing, and dashboard |
| **Desktop** (`apps/desktop/`) | Execution engine — runs actions, workflows, plugins, and device coordination |

## Repository Structure

```
AUTO_MAT_DECK/
├── apps/           # Platform applications (mobile + desktop)
├── packages/       # Shared protocol and resources
├── docs/           # Architecture, decisions, UI docs, release notes
├── tools/          # Developer scripts and templates
└── .github/        # CI/CD workflows
```

## Development

```bash
# Install Task runner: https://taskfile.dev/installation
task build      # Build all apps and packages
task lint       # Run all linters
task test       # Run all tests
task ci         # Full CI pipeline locally
```

See [AI_CONTEXT.md](./AI_CONTEXT.md) for the complete project guide — architecture rules, naming conventions, versioning, and standards every contributor follows.

## License

[MIT](./LICENSE)
