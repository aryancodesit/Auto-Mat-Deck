# ADR-011: Active Profile Projection

**Status:** Accepted
**Date:** 2026-07-13

## Context

v0.3 introduces desktop-authoritative runtime Profiles. The desktop observes
the foreground process, resolves a process-to-Profile rule, maintains the
active Profile, and must communicate the active Profile definition to
connected Android clients so they can render a dynamic command deck.

The question is: what serialization strategy does the `active_profile_state`
protocol message use for the Profile payload?

## Decision

**Use explicit protocol projection DTOs.**

Do not serialize the domain `Profile`/`Page`/`Button`/`ActionReference` types
directly.

Create lightweight projection types in the protocol module:

```rust
// Conceptual — actual path determined during Sprint 3 implementation
struct ActiveProfileState {
    selection_mode: SelectionModeDto,
    profile: Option<ProfileProjection>,
}

enum SelectionModeDto {
    Automatic,
    Manual,
}

struct ProfileProjection {
    id: String,
    name: String,
    pages: Vec<PageProjection>,
}

struct PageProjection {
    id: String,
    name: String,
    buttons: Vec<ButtonProjection>,
}

struct ButtonProjection {
    id: String,
    label: String,
    action: ActionProjection,
}

struct ActionProjection {
    action_name: String,
    payload: serde_json::Value,
}
```

## Rationale

### Direct domain serialization (Rejected)

Serializing the domain `Profile` directly would couple the persistence schema
to the wire protocol. Future domain-only changes (e.g. adding an internal
`notes` field to Profile, or a `sort_order` to Button) would leak to Android
clients and break backward compatibility.

### Projection DTOs (Accepted)

- **Persistence schema and wire protocol are independent.** A domain field can
  be added without becoming a protocol contract field.
- **Explicit contract.** Every field in `active_profile_state` is a deliberate
  choice. Documentation is self-evident — the DTO struct IS the schema.
- **Android independence.** If Android needs a different shape later (e.g.
  paged buttons, image URLs), the DTO can evolve independently of the domain.
- **Serialization cost.** Projection requires a conversion step. This is
  negligible for the current document sizes (dozens of buttons, not thousands).

## Protocol contract

### Message

```json
{
  "type": "active_profile_state",
  "selection_mode": "automatic",
  "profile": {
    "id": "abc123...",
    "name": "Development",
    "pages": [
      {
        "id": "def456...",
        "name": "Main",
        "buttons": [
          {
            "id": "ghi789...",
            "label": "Open Chrome",
            "action": {
              "action_name": "launch",
              "payload": { "app": "chrome" }
            }
          }
        ]
      }
    ]
  }
}
```

### Null profile (no active profile)

```json
{
  "type": "active_profile_state",
  "selection_mode": "automatic",
  "profile": null
}
```

### `selection_mode` serialization

`SelectionModeDto` serializes as a simple string:

| Rust variant | JSON value |
|---|---|
| `Automatic` | `"automatic"` |
| `Manual` | `"manual"` |

Android does not receive the Manual ProfileId — it receives the resolved
Profile projection. Android's concern is "what to render," not "why this
Profile is active."

### Field documentation

| JSON field | Type | Nullable | Description |
|---|---|---|---|
| `type` | string | no | Always `"active_profile_state"` |
| `selection_mode` | string | no | `"automatic"` or `"manual"` |
| `profile` | object | yes | Active Profile projection or `null` |
| `profile.id` | string | no | ProfileId as string |
| `profile.name` | string | no | Human-readable Profile name |
| `profile.pages` | array | no | Pages (may be empty) |
| `pages[].id` | string | no | PageId as string |
| `pages[].name` | string | no | Human-readable page name |
| `pages[].buttons` | array | no | Buttons (may be empty) |
| `buttons[].id` | string | no | ButtonId as string |
| `buttons[].label` | string | no | Display label |
| `buttons[].action` | object | no | Action reference |
| `action.action_name` | string | no | Registered action name |
| `action.payload` | object | no | Action-specific arguments |

## Push conditions

The `active_profile_state` message is sent when:

1. **Initial authorization** — After sending `"trusted"` or `"pair_accepted"`
   response to a newly connected client.
2. **Active Profile changes** — Automatic switch or manual selection.
3. **SelectionMode changes** — Even if the same Profile remains active.

The message is NOT sent on every WebSocket frame. It is semantically
state-synchronization (idempotent), not a stream of events.

## What must NOT appear in the payload

- Foreground process name or any Win32 detail
- `ContextRule` data or resolver internals
- Full Document (profiles the user did not select)
- `SelectionMode` internal variant data (e.g. the stored `ProfileId`)
- Debug/runtime metadata

## Action execution remains unchanged

The projected `action` fields (`action_name` + `payload`) match the existing
Android action request format exactly:

- Domain `ActionReference` serializes to `{"action_name": "launch", "payload": {"app": "chrome"}}`
- Android `action` request uses `{"type": "action", "action": "launch", "payload": {"app": "chrome"}}`

The only difference is the top-level wrapper type. The projection DTO's
`action_name` maps directly to the `action` field of the existing
`action` message.

This means Android reads `action.action_name` from the projected profile,
constructs:

```json
{
  "type": "action",
  "action": "launch",
  "payload": { "app": "chrome" }
}
```

And sends it through the existing WebSocket. The Desktop `ActionRegistry`
executes it. No new action-execution path is required.
