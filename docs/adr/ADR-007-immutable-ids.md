# ADR-007: Persistent entities use immutable IDs

**Status:** Implemented
**Date:** 2026-07-10
**Applies To:** v0.2+
**Supersedes:** —
**Superseded By:** —
**Engineering Phase:** EP-004
**Release:** v0.2

## Context

Persistent entities (profiles, pages, buttons, devices, actions) need to be
referenced across sessions, in serialized documents, and by external clients.
Using human-readable names as identifiers creates several problems:

- Renaming a profile breaks all references to it.
- Two profiles can accidentally share the same name.
- External clients cannot reliably address an entity whose identifier
  may change.

## Decision

Every persistent entity carries an **immutable, auto-generated ID**.

- IDs are generated once at creation time and never change.
- IDs are opaque strings (hex-encoded timestamp).
- Names are editable presentation fields, not identifiers.
- External references (protocol messages, inter-entity links) always use IDs,
  never names.

### Entities with immutable IDs

| Entity | ID type | Scope | Notes |
|--------|---------|-------|-------|
| Profile | `ProfileId` | Document | Top-level grouping |
| Page | `PageId` | Parent Profile | Contains buttons |
| Button | `ButtonId` | Parent Page | Individual action trigger |
| Device | `DeviceId` | Document | Paired remote device |
| Action | `ActionReference` | — | References by name (registry), not ID |

Note: `ActionReference` uses an action name rather than an ID because actions
are registered at compile time, not created at runtime. If actions become
user-definable in the future, they will receive IDs at that point.

### ID scope rules

- **Document-scoped** IDs (`ProfileId`, `DeviceId`) must be unique across the
  entire Document. Lookup operations search the top-level Document collections.
- **Parent-scoped** IDs (`PageId` within a Profile, `ButtonId` within a Page)
  are guaranteed unique only within their parent container. The reducer always
  resolves them by searching within the parent (e.g., `p.pages.iter().find(|pg|
  pg.id == page_id)`), never by scanning the Document globally.
- These scope rules mean two profiles can legally contain pages with the same
  `PageId` string value without conflict. In practice, callers generate unique
  IDs for all entities to simplify debugging and protocol messages.

### Required initial entities

- A new Document always starts with one Profile containing one Page
  (enforced by `Document::empty()`).
- A new Profile always starts with one Page (enforced by `apply()`, which
  uses the caller-supplied `initial_page_id`).
- A new Page starts with zero Buttons.
- These invariants are enforced by the Command/Reducer.

## Consequences

- **Positive:** Renaming never breaks references. External clients can
  address entities reliably. Schema migrations are simpler — IDs are stable
  across format changes.
- **Negative:** Slightly more complex creation code (generate ID, then set
  initial name). IDs in log output are less readable than names.
- **Neutral:** IDs are UUID-adjacent but not UUID-standard. The project can
  migrate to proper UUIDs later without changing the abstraction.

## Rationale

- This is a well-established pattern (see: relational database primary keys,
  Stream Deck profile IDs, Figma node IDs).
- It is much cheaper to add IDs upfront than to retrofit them once
  references have accumulated.
- Even in a personal-local tool with one user, IDs prevent the common
  "rename and break" mistake.
