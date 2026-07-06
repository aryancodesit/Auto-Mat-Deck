# packages/shared/

Shared **resources** — not shared code.

This directory contains assets and definitions that are consumed by both `apps/mobile/` and `apps/desktop/`, but are **language-agnostic** and contain **no runtime code**.

## Contents

| Item | Description |
|------|-------------|
| `icons/` | Application icons and UI icon set |
| `profiles/` | Default profile templates |
| `workflows/` | Sample workflow definitions |
| `schemas/` | JSON Schema files for validation |
| `localization/` | Translation files (strings, locales) |
| `rules/` | Validation rule definitions shared across apps |

## Why resources instead of code?

- Avoids coupling mobile and desktop to a shared runtime dependency.
- Resources can be consumed by any language — Kotlin, Rust, Go, C#, etc.
- Changes to shared resources don't require recompilation of both apps.
- Keeps `packages/` focused on the protocol contract as the only true shared code dependency.

## Usage

- Mobile app imports resources at build time (e.g., Gradle resource processing).
- Desktop agent reads resources from the filesystem or embeds them at build time.
- Protocol references (e.g., error codes, field enums) live in `packages/protocol/`, not here.
