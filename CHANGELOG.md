# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Monorepo structure with apps/, packages/, docs/, tools/, .github/
- Architecture documentation: Overview, Protocol, Discovery, Pairing & Authentication, Synchronization, Security Model, Core Domain Model, Profiles, Triggers, Plugin System
- Architectural Decision Records: monorepo structure, protocol-first, independent versioning, trunk-based development, synchronization model, transport evaluation
- spikes/ directory for disposable validation prototypes
- Spike ep-001: discovery, pairing, and transport validation on real devices
- Taskfile.yml for cross-platform build orchestration
- AI_CONTEXT.md as the single source of truth for AI tools
- GitHub Actions CI workflow (placeholder)
- MIT License, CHANGELOG.md, .gitignore

### Changed
- packages/shared redefined as shared resources (not runtime code)
- Versioning decoupled: mobile, desktop, and protocol version independently
- Branch strategy simplified to trunk-based (main + feature/*)
- Desktop agent redefined: no operational UI, minimal bootstrap UI for pairing only
- Protocol document updated to reference transport ADR and evaluation criteria
- Roadmap extended with v0.1.5 validation spike milestone

### Removed
- packages/sdk (YAGNI — deferred to v1.x)
- develop branch from strategy
- Python-centric scaffold replaced with platform-accurate monorepo
