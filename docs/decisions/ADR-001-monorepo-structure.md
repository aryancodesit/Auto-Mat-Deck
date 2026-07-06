# ADR-001: Monorepo Structure

**Status:** Accepted
**Date:** 2026-07-06

## Context

Auto-Mat-Deck is a multi-application platform with shared protocol definitions, domain models, and documentation. We need a repository structure that enables independent development of mobile and desktop applications while maintaining a single source of truth for shared artifacts.

## Decision

Adopt a monorepo with the following top-level directories:

- `apps/` — Platform-specific applications (mobile, desktop)
- `packages/` — Shared protocol definitions and shared resources
- `docs/` — Architecture, decisions, UI documentation, release notes
- `tools/` — Developer scripts and templates

`packages/protocol/` contains the communication protocol schema and message definitions. `packages/shared/` contains shared resources (icons, default profiles, sample workflows, schemas, localization). No SDK package is included — it will be introduced at v1.x if the plugin API stabilizes.

## Rationale

- Single source of truth for the protocol contract
- Simplified cross-referencing between apps and shared packages
- Coordinated builds via CI path triggers
- Easier onboarding — everything is in one place

## Consequences

- Requires disciplined CI to avoid building/testing everything on every change
- Path-based CI triggers needed per app/package
- Monorepo tooling (or careful scripting) required for dependency management
