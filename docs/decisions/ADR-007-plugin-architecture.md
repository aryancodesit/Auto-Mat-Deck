# ADR-007: Plugin Architecture (Deferred)

**Status:** Proposed
**Date:** 2026-07-06

## Context

The desktop agent's action library is intended to be plugin-based. The architecture documents reference plugins as a core concept, but several critical questions remain unanswered:

- How are plugins discovered? (Directory scan, manifest file, registry?)
- What permissions do they have? (Full system access, sandboxed, capability-gated?)
- Can they execute arbitrary code? (Native binaries, scripts, WASM?)
- How are they sandboxed? (Process isolation, language VM, container?)
- How are they updated? (Auto-update, manual replacement, package manager?)
- How are they trusted? (Code signing, hash verification, user approval?)
- How are they versioned alongside the agent? (Compatibility matrix, API versioning?)
- What is the plugin API surface? (Stable ABI, IPC protocol, FFI boundary?)

These questions cannot be answered until the core agent is built and we understand what actions actually look like in practice.

## Decision

Defer all plugin architecture decisions until after v0.6 (Actions milestone). Do not design the plugin system until we have:

1. A working desktop agent skeleton (v0.2).
2. A clear understanding of what actions the first plugins must support.
3. Real experience executing actions via the agent.

Until then, the architecture acknowledges plugins as a concept but makes no commitments about discovery, sandboxing, trust, or API design.

## Rationale

- Premature plugin architecture would constrain the core agent design before we know what constraints matter.
- Building plugins too early risks designing for hypothetical use cases that never materialize.
- The agent can initially ship with built-in actions; the plugin interface can be extracted later.

## Consequences

- The first desktop agent (v0.2) will use built-in actions only, not plugins.
- Plugin capability is explicitly deferred — not forgotten, not accidentally designed.
- ADR-007 will be revisited after v0.6 with concrete requirements.
- Plugin-related questions in architecture docs remain as open items, not commitments.
