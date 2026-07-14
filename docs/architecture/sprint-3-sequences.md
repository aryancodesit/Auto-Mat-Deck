# Sprint 3 — Sequence Diagrams

**Status:** Draft
**Date:** 2026-07-13

Normative references: Sprint 3 Scope, ADR-014, ADR-015, ADR-016.

---

## 1. Normal flow — context change triggers projection

```
Observer Thread           Projection Thread              Publisher         Transport
      │                         │                            │                 │
      │  lock(DT)               │                            │                 │
      │  apply_context_obs()    │                            │                 │
      │  unlock(DT)             │                            │                 │
      │  store(LatestTransition)│                            │                 │
      │  notify()               │                            │                 │
      │────────────────────────►│                            │                 │
      │                         │  read(LatestTransition)    │                 │
      │                         │  project(transition)       │                 │
      │                         │  should_publish(proj)      │                 │
      │                         │  publish(proj)             │                 │
      │                         │───────────────────────────►│                 │
      │                         │                            │  write(proj)    │
      │                         │                            │────────────────►│
      │                         │                            │                 │
```

Conditions:
- `context_changed == true`
- `should_publish` returns `true` (projection differs from previous)

---

## 2. Deduplicated observation — no broadcast

```
Observer Thread           Projection Thread              Publisher
      │                         │                            │
      │  lock(DT)               │                            │
      │  apply_context_obs()    │                            │
      │  unlock(DT)             │                            │
      │  store(LatestTransition)│                            │
      │  notify()               │                            │
      │────────────────────────►│                            │
      │                         │  read(LatestTransition)    │
      │                         │  project(transition)       │
      │                         │  should_publish(proj)      │
      │                         │  (suppress: identical)     │
      │                         │                            │
```

Conditions:
- `context_changed == true`
- `should_publish` returns `false` (projection == previous emitted)

---

## 3. Lossy overwrite — projection thread lagging

```
Observer                    Projection (busy)           Storage
      │                         │                         │
      │  store(T1)              │                         │  T1
      │  notify()               │                         │
      │────────────────────────►│ (still processing T0)   │
      │                         │                         │
      │  store(T2)              │                         │  T2 (overwrites T1)
      │  notify()               │                         │
      │                         │                         │
      │                         │  read() → T2            │
      │                         │  (T1 was lost)          │
      │                         │                         │
```

Conditions:
- Publisher blocked on I/O during T0–T1 interval
- T1 never read — T2 replaces it before the projection thread wakes

---

## 4. Publisher failure — silent swallow

```
Observer                  Projection              Publisher
      │                       │                       │
      │  notify()             │                       │
      │──────────────────────►│                       │
      │                       │  read()               │
      │                       │  project()            │
      │                       │  publish()            │
      │                       │──────────────────────►│  write fails
      │                       │                       │  ──┬──
      │                       │                  error│  │ swallow
      │                       │                       │◄─┘
      │                       │  (continues loop)     │
      │  notify(next)         │                       │
      │──────────────────────►│                       │
      │                       │  publish(next)        │
      │                       │──────────────────────►│  write succeeds
      │                       │                       │
```

Conditions:
- Transient write error
- Publisher does not panic — next publication may succeed
- Projection thread never sees the error

---

## 5. Shutdown sequence

```
Main Thread            Projection Thread          Observer Thread
      │                       │                        │
      │  signal shutdown      │                        │
      │──────────────────────►│                        │
      │                       │  exit loop             │
      │                       │  (thread terminates)   │
      │                       │                        │
      │  join()               │                        │
      │◄──────────────────────│                        │
      │                       │                        │
      │  drop(runtime_ref)    │                        │
      │───────────────────────────────────────────────►│
      │                       │                        │  shutdown watch fires
      │                       │                        │  exit loop
      │                       │                        │  (thread terminates)
      │───────────────────────────────────────────────►│
      │  join()              │                        │
      │◄───────────────────────────────────────────────│
```

Ordering invariants:
1. Projection thread exits before observer thread.
2. Observer thread's runtime reference is dropped after projection thread joins.
3. All threads joined before `main()` returns.

---

## 6. Manual mode — context recorded, profile unchanged

```
Observer Thread           Projection Thread
      │                         │
      │  store(transition)      │
      │  notify()               │
      │────────────────────────►│
      │                         │  read()
      │                         │  project()
      │                         │  result: active_profile_id unchanged
      │                         │  should_publish() → depends on
      │                         │  projection vs previous emitted
```

Conditions:
- `selection_mode == Manual(ProfileId)`
- `context_changed == true`
- `active_profile_id` not affected by context observation
- `latest_context` updated
- Publication policy compares full projection, not just profile change

---

## 7. No profiles — projection with active_profile_id == None

```
Observer Thread           Projection Thread           Publisher
      │                         │                        │
      │  store(transition)      │                        │
      │  notify()               │                        │
      │────────────────────────►│                        │
      │                         │  read()                │
      │                         │  project()             │
      │                         │  (active_profile: None)│
      │                         │  should_publish()      │
      │                         │  publish(None)         │
      │                         │───────────────────────►│
```

Conditions:
- `profiles` is empty
- Transition fires — publication carries `None` as active profile
- Downstream consumer learns: no profile to display
