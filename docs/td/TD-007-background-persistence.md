# TD-007: Background persistence service

**Priority:** Medium
**Target:** EP-004 Sprint 3 or later

## Description

Currently, every write (forget device, rename, etc.) blocks the GUI thread
with a synchronous JSON write to disk. This is acceptable for v0.2 with
~10 devices, but will cause noticeable stalls as the editor grows to
include pages, buttons, icons, layouts, and workflows.

## Recommendation

Introduce a background persistence service:

```
GUI / Agent
    │
    ▼
AppState (in-memory, immediate)
    │
    ▼
Background persistence channel
    │
    ▼
Debounced writer → disk (async)
```

The writer should debounce rapid mutations (e.g., drag-and-drop
reordering triggers 10 saves in 1 second) and batch them into a single
write.

## Do not implement yet

Synchronous writes are fine for the current data volume. Defer this until
the editor produces enough mutations to make stalls observable.
