# ADR-009: Context Rules Belong to Document

**Status:** Accepted
**Date:** 2026-07-13

## Context

v0.3 introduces foreground-process context awareness. When the user switches
to a known application window (e.g. `Code.exe`), the desktop should activate
the corresponding Profile automatically.

The question is: where does the mapping from "process name" to "Profile ID"
live?

Two candidates were evaluated:

1. **ContextRule as a Profile field** — `Profile { ..., context_rules: Vec<ContextRule> }`
2. **ContextRule as a Document field** — `Document { ..., context_rules: Vec<ContextRule> }`

## Decision

**ContextRule belongs to Document, not Profile.**

```rust
// Accepted schema
pub struct Document {
    pub schema: u32,
    pub created_with: String,
    pub last_saved_with: String,
    pub devices: Vec<TrustedDevice>,
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub context_rules: Vec<ContextRule>,   // NEW
}
```

The `#[serde(default)]` annotation is mandatory. The current
`JsonRepository::load()` path uses direct serde deserialization with no schema
validation, migration layer, or missing-field recovery outside serde defaults.
A v0.2 `document.json` does not contain a `context_rules` field. Without this
annotation, deserialization would fail on upgrade.

## Schema version policy

Document schema remains at `1` for v0.3.

- Existing v0.2 schema-1 documents load with `context_rules = []`.
- On save, `context_rules` is serialized.
- `schema` field remains `1`.
- `last_saved_with` follows existing repository behaviour (set to
  `env!("CARGO_PKG_VERSION")` at `Document::empty()` creation; not
  automatically bumped on save — `JsonRepository::save()` is a pass-through
  serialize. A v0.2 document loaded and saved by v0.3 retains `"0.2.0"`.
  Automatic bumping is deferred to a schema-versioning ADR.)

A proper Document schema versioning and migration strategy is deferred to a
dedicated future architecture decision. v0.3 does not introduce a migration
framework.

The ContextRule shape:

```rust
pub struct ContextRule {
    pub id: ContextRuleId,
    pub process_name: String,   // stored normalized (trimmed, lowercased)
    pub profile_id: ProfileId,
}
```

No `created_at`. No `priority`. No `enabled` flag. No metadata beyond the
mapping itself. YAGNI applies — v0.3 has no requirement for any of these.

## Rationale

A ContextRule expresses the relationship:

"When environment condition X is true, activate Profile Y."

It is a **connection** between context and a Profile, not an **intrinsic
property** of the Profile itself. Placing rules in Document keeps them
alongside both sides of the relationship.

This also prevents future Profile pollution:

```
// What would accumulate if rules lived on Profile:
Profile {
    foreground_process  // Wrong — there could be multiple processes per profile
    window_title        // Deferred to later version
    wifi_ssid           // Deferred
    idle_state          // Deferred
    time_range          // Deferred
}
```

ContextRule is the correct domain boundary. Future signal types
(window-title, network, idle, time) all get additional rule variants without
polluting Profile.

## Consequences

- **Positive:** Document is the single persistence unit for all configuration.
  Rules are serialized, deserialized, and migrated alongside profiles.
- **Positive:** Deleting a Profile cascade-deletes its rules (same reducer
  mutation) — no orphan rules.
- **Positive:** Future editor UI for rules sits alongside Profile/Page/Button
  editors in the same tab.
- **Negative:** Rules are not "attached" to a Profile in the data model.
  The editor must present them by filtering `document.context_rules` by
  `profile_id`.
- **Neutral:** The `profile_id` foreign key is checked at insertion time.
  Deletion cascading ensures integrity.

## Process normalization

A single domain-level function provides normalization for both insertion
validation and resolver matching:

```rust
pub fn normalize_process_name(name: &str) -> String {
    name.trim().to_lowercase()
}
```

Semantics are: trim, case-insensitive exact match. No substring matching.
No regex.

The Windows observer reports observed process identity (e.g. `"Code.exe"`).
The resolver applies `normalize_process_name` before comparing. The editor
stores the normalized form so duplicate detection in the reducer is also
normalized comparison.

## Duplicate policy

Duplicate normalized process names are invalid configuration. The reducer
rejects them with `CommandError::DuplicateContextRuleProcess`.

Do not use first-match-wins as implicit rule priority. Priority-based
resolution is explicitly deferred to a future version.

## Referential integrity on Profile deletion

The `DeleteProfile` reducer arm cascade-deletes all `ContextRule` entries
with a matching `profile_id` within the same reducer mutation. The caller
does not need to manually remove rules before deleting a Profile.
