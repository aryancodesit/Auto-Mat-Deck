# Spike ep-004: Configuration Ownership & Multi-Device Behavior

> Validate that mobile remains the authoritative source of configuration when multiple devices are involved.

## Question

Does mobile remain authoritative when multiple phones or desktops interact? Can two phones coexist? What happens when a phone edits configuration that another phone already changed?

## Scope

- One desktop, two mobiles: concurrent pairing, configuration authority, conflict behavior.
- One mobile, two desktops: switching between desktops, profile management.
- Identity collision: two desktops with the same hostname on the same network.
- Re-pairing after device replacement (new phone, new desktop).
- Desktop identity stability across OS reinstall.
- Mobile edits configuration while desktop is offline → desktop receives changes on reconnect.
- Second mobile pairs with same desktop → does it see the existing configuration?

## Out of Scope

- Distributed synchronization algorithms (CRDTs, merge strategies). Mobile always wins.
- Bidirectional sync — desktop never pushes configuration changes to mobile.
- Transport or encryption (covered by EP-001).
- Android or Windows lifecycle (covered by EP-002, EP-003).

## Success Thresholds

| Metric | PASS | WARNING | FAIL |
|--------|------|---------|------|
| Two mobiles pair with same desktop | Both pair cleanly | Conflicts resolved manually | Pairing breaks |
| Mobile edits config while desktop offline | Desktop receives on reconnect | Partial sync | Data loss |
| Second mobile pairs | Sees current config | Sees stale config | Cannot pair |
| Desktop re-pair after OS reinstall | Fresh pairing, old identity rejected | Manual cleanup needed | Cannot re-pair |

## Status

Planned. Not started until EP-001 and EP-003 complete.
