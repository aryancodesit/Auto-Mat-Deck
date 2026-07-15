# ADR-021: Control Surface Projection

**Status:** Accepted
**Date:** 2026-07-15

## Context

ADR-020 established the `active_profile_state` projection: a Mobile-facing,
Desktop-authoritative, latest-value snapshot of the active Profile identity.
v0.4 delivers this single-field projection to paired Android clients.

Mobile users need to see what controls are available for the active Profile —
not just the Profile name, but the full hierarchy of Pages and their Buttons,
with display labels. The Desktop holds this data in the Profile document.
The Mobile client has no direct access to the Profile store.

v0.5 must project the active Profile's control surface to paired Mobile
clients as a second independent projection stream, preserving all v0.4
architectural guarantees.

## Certified Projection Boundary

The authoritative repository domain model (model.rs) defines:

```
Profile
├── id: ProfileId (timestamp-hex String newtype)
├── name: String (display name)
└── pages: Vec<Page>
      ├── id: PageId (timestamp-hex String newtype)
      ├── name: String (display name)
      └── buttons: Vec<Button>
            ├── id: ButtonId (timestamp-hex String newtype)
            ├── label: String (display label)
            └── action: ActionReference (Desktop-internal, excluded from projection)
```

Uniqueness evidence (from command.rs):
- ButtonId uniqueness is enforced **per-page only** (AddButton, MoveButton checks)
- **No cross-page or cross-profile ButtonId uniqueness**
- **No PageId uniqueness enforcement**
- All IDs are timestamp-hex String newtypes, not UUID

Ordering:
- Page order = Vec<Page> index
- Button order = Vec<Button> index
- No explicit sort/order fields

This ADR documents the projection contract that communicates this hierarchy
to Mobile clients, with the exact proven uniqueness semantics.

## Decision Drivers

1. **Two independent projections** — `active_profile_state` (v0.4, frozen) and
   the new control-surface projection are separate messages. They share the
   same delivery infrastructure (watch channel, WebSocket) but are semantically
   independent.
2. **Latest-value state, not event stream** — the control-surface projection
   follows the same lossy, newest-authoritative semantics established in
   ADR-015 and ADR-020. The Mobile client receives a complete snapshot of the
   current control surface, not a log of changes.
3. **Profile → Pages → Buttons hierarchy** — the projection must preserve
   the certified three-level structure. Flattening to a single controls array
   removes Page grouping and ordering, which violates the domain model.
4. **No raw `ActionReference` on Mobile** — the existing `ActionReference`
   type (action_name + payload) is a Desktop-internal domain type. The wire
   format must not leak Desktop domain types. Mobile receives projected
   control identities and display labels only.
5. **Structural identity and display presentation only** — v0.5 projects
   what exists and how it is named. No execution-derived state (assigned,
   enabled, configured, has_action) is projected in v0.5.
6. **ButtonId projected as opaque stable control identity** — Mobile
   references controls by ButtonId when sending action requests (future).
   The Desktop maps ButtonId → ActionReference internally.
7. **Profile association** — the control-surface projection is explicitly
   associated with a Profile (profile_id, profile_name). A Mobile client
   eligible for the active-profile projection is also eligible for the
   control-surface projection (same trust and eligibility gate).
8. **Explicit no-active and empty-surface semantics** — three cases are
   distinguished: no active Profile (null triple), active Profile with
   zero Pages (pages=[]), and unresolved active Profile (derivation failure).
9. **No atomic combined delivery** — the two projections are independent.
   No cross-projection ordering guarantee exists. APS may arrive before CSS
   or vice versa. Mobile must tolerate temporary mismatch.
10. **`schema_version`** — the control-surface message carries its own
    `schema_version` field, starting at 1 for v0.5. No projection revision
    counter is introduced in v0.5.
11. **Derivation from authoritative Document + active Profile identity** —
    the control-surface projection is derived from the active Profile's
    Document entry. Publication only when derived value changes.

## Considered Alternatives

### A. Extend `active_profile_state` with control surface data — REJECTED

Merge the control-surface fields into the existing `active_profile_state` v1
message. Rejected because `active_profile_state` v1 is frozen and the two
concepts are architecturally independent.

### B. Single watch channel carrying both projections — REJECTED

One `watch::Sender` that serializes both into a single frame. Rejected because
it couples failure modes and prevents independent evolution.

### C. Two watch channels — SELECTED

Each projection has its own `watch::Sender` and `watch::Receiver` chain. The
connection task clones both receivers and forwards both independently. This
is the minimal additive implementation given the v0.4 infrastructure
(ADR-020). No PublisherHub or other abstraction is introduced.

### D. Delta/event-based projection — REJECTED

Individual button-add/remove/update events. Rejected per ADR-015 latest-value
semantics. Full snapshot is smaller than the state management logic needed for
deltas given the expected scale.

### E. Raw `ActionReference` in the wire format — REJECTED

Leaks Desktop domain types. The Desktop maps ButtonId → ActionReference
internally. Mobile never sees ActionReference.

### F. Flattened controls array — REJECTED

The certified domain model has three levels (Profile → Pages → Buttons).
A flattened controls array eliminates Page grouping, Page identity, Page
ordering, and per-Page Button ordering — all certified projection
requirements.

## Decision

### Message

A new wire message type `control_surface_state` is added to the protocol.

#### Conceptual DTO hierarchy

```
ControlSurfaceState
├── type: "control_surface_state"
├── schema_version: 1
├── profile_id: string or null
├── profile_name: string or null
└── pages: array of PageProjection, or null
      ├── page_id: string (opaque identity)
      ├── name: string (display name)
      └── buttons: array of ButtonProjection
            ├── button_id: string (opaque identity)
            └── label: string (display label)
```

#### Payload contract (v1)

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | yes | Always `"control_surface_state"` |
| `schema_version` | integer | yes | The exact integer `1` for v1 |
| `profile_id` | string or null | yes | The Profile ID this surface belongs to, or null if no Profile active |
| `profile_name` | string or null | yes | The Profile display name, or null if no Profile active |
| `pages` | array or null | yes | Array of PageProjection, or null if no Profile active |

##### PageProjection

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `page_id` | string | yes | Opaque stable Page identity |
| `name` | string | yes | Page display name |
| `buttons` | array | yes | Array of ButtonProjection |

##### ButtonProjection

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `button_id` | string | yes | Opaque stable button identity |
| `label` | string | yes | Button display label |

**Identity semantics:**
- ButtonId is a timestamp-hex String newtype (model.rs). The projection
  communicates the string value as-is. The projection does not rely on
  Document-global ButtonId uniqueness — the authoritative model enforces
  ButtonId uniqueness per-page only.
- PageId is a timestamp-hex String newtype. The authoritative model does not
  enforce PageId uniqueness. The projection communicates PageId values as-is
  for stable reference.
- Neither ButtonId nor PageId identity should be treated as globally unique
  by Mobile unless repository enforcement of that invariant is independently
  verified.

**Order semantics:**
- Page order: the array order reflects the authoritative Vec<Page> order.
- Button order within each Page: the array order reflects the authoritative
  Vec<Button> order.

**Examples:**

Profile "Coding" with one Page containing two Buttons:
```json
{
  "type": "control_surface_state",
  "schema_version": 1,
  "profile_id": "0000000000000000a1b2c3d4e5f6a7b8",
  "profile_name": "Coding",
  "pages": [
    {
      "page_id": "00000000000000001122334455667788",
      "name": "Main",
      "buttons": [
        { "button_id": "0000000000000000aabbccddeeff0011", "label": "Compile" },
        { "button_id": "00000000000000002233445566778899", "label": "Test" }
      ]
    }
  ]
}
```

No active Profile (null triple):
```json
{
  "type": "control_surface_state",
  "schema_version": 1,
  "profile_id": null,
  "profile_name": null,
  "pages": null
}
```

Active Profile with zero Pages:
```json
{
  "type": "control_surface_state",
  "schema_version": 1,
  "profile_id": "0000000000000000a1b2c3d4e5f6a7b8",
  "profile_name": "Minimal",
  "pages": []
}
```

### Delivery architecture

#### Channel topology

```
DesktopRuntime
      │
      ▼
ProjectionEngine
      │
      ├── PublicationPolicy (active_profile_state)
      │         │
      │         ▼
      │   ProjectionTransportPublisher (active_profile_state)
      │         │
      │         ▼
      │   watch::Sender<Option<Arc<str>>>  ─── channel A
      │
      └── PublicationPolicy (control_surface_state)
                │
                ▼
          ProjectionTransportPublisher (control_surface_state)
                │
                ▼
          watch::Sender<Option<Arc<str>>>  ─── channel B
```

Each channel is independent. Two watch channels are selected because they
are the minimal additive implementation given the v0.4 watch infrastructure
(ADR-020). No PublisherHub or shared abstraction is introduced.

#### Receiver topology

```
handle_connection task
      │
      ├── Receiver A (cloned from channel A)
      │     └── changed() → send over WebSocket as "active_profile_state"
      │
      └── Receiver B (cloned from channel B)
            └── changed() → send over WebSocket as "control_surface_state"
```

Each `handle_connection` task clones both receivers. Both branches run in
the same `select!` loop. Each branch handles its own `changed()` notification
independently. No cross-channel ordering guarantee exists.

### Trust and eligibility

Same trust gate as v0.4 applies to both projections:
- Untrusted connection: neither receiver is cloned — no projection state
  reaches the connection
- Trusted connection: both receivers are cloned after `trusted` /
  `pair_accepted`

### Invalidation policy

The control-surface projection is derived from the active Profile's Document
entry. The projection must be recomputed when the derived value changes.

**Triggers that invalidate CSS:**
- Active Profile identity changes (profile_id differs)
- Active Profile display name changes
- Page added, renamed, or removed
- Button added, updated (label), removed, or reordered
- Button identity changes (should not occur in practice — stable identity)

**Triggers that do NOT invalidate CSS:**
- Inactive Profile edit
- Context observation without active Profile change
- Trust state changes unrelated to projection derivation

**No-op suppression:** If the derived CSS value equals the previously
published value, no frame is sent.

**Unresolved active Profile:** If active_profile_id is Some(id) but the
authoritative Document/configuration cannot resolve that Profile, the
derivation function fails. No fabricated empty projection is published.
The retained Desktop CSS channel is unchanged. The event is logged.

### Serialization boundary

```
Profile Document (Desktop store)
      │
      ▼
Derived ControlSurfaceState (projection domain type)
      │
      ▼
control_surface_state DTO (serde::Serialize)
      │
      ▼
serde_json::to_string()
      │
      ▼
Arc<str> → watch::Sender::send_replace()
```

- Serialization occurs once, inside the CSS publisher's `publish()`
- Serialization failure: logged, drop, channel unchanged (matches ADR-020)
- The DTO is a separate type — no Desktop domain types leak

### Nullability invariants

| profile_id | profile_name | pages | Meaning |
|------------|-------------|-------|---------|
| null | null | null | No active Profile — surface unavailable |
| string | string | array | Active Profile with surface (may be empty array) |

If profile_id is null:
  → profile_name MUST be null
  → pages MUST be null

If profile_id is a string:
  → profile_name MUST be a string (may be empty)
  → pages MUST be an array (may be empty)

Any combination violating these invariants is structurally invalid.

`pages: null` always means "no active Profile." An active Profile with zero
Pages uses `pages: []`. These are architecturally distinct.

## Association and Eligibility Semantics

Mobile independently retains the latest valid:

- `ActiveProfileStateMessage` (APS) — active Profile identity
- `ControlSurfaceStateMessage` (CSS) — control surface snapshot

Eligibility is derived:

| retained APS (active_profile_id) | retained CSS (profile_id) | CSS eligible as current? |
|----------------------------------|--------------------------|--------------------------|
| non-null, matches CSS profile_id | valid | yes |
| non-null, differs from CSS profile_id | valid | no (temporary mismatch) |
| null | any | no |
| any | null (no active Profile) | no |

A structurally valid CSS frame always replaces the retained latest CSS state,
regardless of whether it matches the current APS profile_id. A mismatching
CSS is retained as valid but ineligible.

This means:
- APS(gaming) then CSS(coding): retained CSS = coding surface, eligible = none
- CSS(coding) then APS(gaming): same final state, convergence is safe
- CSS(coding) then CSS(gaming): retained CSS = gaming, eligible = gaming
  (convergence achieved regardless of APS ordering)

Temporary mismatch is architecturally valid. No atomic combined delivery
is required.

## Consequences

### Positive

- Fidelity to the certified Profile → Pages → Buttons hierarchy
- Profile association explicit (profile_id + profile_name)
- Structural identity and display presentation only — no execution state leak
- ButtonId projected as opaque stable identity
- Latest-value full snapshot semantics preserved
- No-active, empty-surface, and unresolved-Profile cases distinguished
- Eligibility derived independently from retained state
- Temporary mismatch tolerated — convergence safe from either order
- Two independent watch channels with independent failure modes
- `active_profile_state` v1 unchanged

### Negative

- Connection task manages two receivers instead of one
- Android parser must handle three-level nesting
- Mobile must independently retain APS and CSS for eligibility derivation

### Neutral

- Two watch channels are the minimal additive implementation — no new
  infrastructure beyond v0.4
- No projection revision counter — schema_version suffices for v0.5
- Future Desktop-authoritative action processing remains an independent
  concern

## Compatibility

- `active_profile_state` v1: unchanged, frozen, unaffected
- Existing Mobile clients ignoring unknown types: unaffected
- Existing Desktop code: no changes to v0.4 projection infrastructure
- ADR-014, ADR-015, ADR-016, ADR-020: all preserved
- WebSocket protocol: one new message type added
- Existing inbound action handler: unchanged
- `action_result`: unchanged

## Resolved Questions

1. **Two independent watch channels?** Yes. Minimal additive implementation.

2. **What is the certified hierarchy?** Profile → Pages → Buttons, per the
   authoritative domain model (model.rs).

3. **What is the uniqueness invariant?** ButtonId uniqueness enforced per-page
   only; no Document-global uniqueness. PageId uniqueness not enforced.
   The projection communicates IDs as-is.

4. **Should `pages: null` vs `pages: []` be distinguished?** Yes. null = no
   active Profile. [] = active Profile with zero Pages.

5. **Should Mobile discard a mismatching CSS?** No. Retain as valid but
   ineligible. Temporary mismatch is architecturally valid.

6. **Does the protocol require APS before CSS?** No. Independent projections.
   Mobile must tolerate either arrival order.

7. **Should assigned be projected?** No. It is execution-derived state, not
   structural identity or display presentation. Out of scope for v0.5.

8. **What if the active Profile cannot be resolved?** Derivation failure.
   No fabricated empty surface. The CSS channel is unchanged.

## Open Questions

None. All design decisions resolved per the Governor-certified architecture.
