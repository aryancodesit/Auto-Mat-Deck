# ADR-023: Workflow Engine

**Status:** Proposed
**Date:** 2026-07-16
**Supersedes:** —
**Superseded By:** —

## Context

v0.5 established a complete invocation loop: Android sends an opaque
button_id, Desktop validates and executes a single atomic action, and
returns the result. This works well for single actions but cannot
express multi-step automation sequences.

Users need to compose multiple actions into reusable sequences —
"launch Chrome, open DevTools, lock the workstation" — triggered by
a single tap. The question is where the workflow engine belongs in
the existing architecture.

## Decision

### 1. Workflows are execution-layer objects, not transport features

A workflow is a first-class executable object owned entirely by the
Desktop. It is not a new protocol capability — it is a new execution
target that the existing protocol can invoke opaquely.

```
Android invocation
        │
        ▼
Desktop Validation
        │
        ▼
Execution Target
      ├──────────────┐
      │              │
 Single Action   Workflow
      │              │
      └──────┬───────┘
             ▼
    ActionRegistry.execute()
             ▼
      Operating System
```

### 2. Reuse control_invoke — no new message types

Android continues to send `control_invoke` with an opaque button_id.
The Desktop resolves whether the button maps to an Action or a
Workflow. Android never needs to distinguish between them.

This preserves the opacity principle from ADR-022. Adding a dedicated
`workflow_invoke` message would expose workflow semantics to Android,
coupling the transport layer to execution internals.

### 3. WorkflowStep references ActionId, not action_name

```rust
pub struct WorkflowStep {
    pub action_id: ActionId,        // immutable identifier
    pub payload: Option<Value>,     // optional action parameters
}
```

Names belong in projection and UI. Identifiers belong in the domain.
Referencing by ActionId means renaming an action (e.g. "launch" →
"open_app") does not break stored workflows.

### 4. WorkflowVersion is included from the start

```rust
pub struct WorkflowVersion(pub u16);  // initially 1
```

Even though only version 1 exists, the field is part of the domain
model from the beginning. Adding it later would require a serialization
migration. Including it now costs nothing and future-proofs the format.

### 5. Workflows live in Document

```
Document
  ├── profiles: Vec<Profile>
  ├── context_rules: Vec<ContextRule>
  └── workflows: Vec<Workflow>
```

This follows ADR-001 (Desktop owns configuration) and ADR-008
(Command/Reducer pattern). Workflows flow through the same mutation
pipeline, persistence layer, and reconciliation logic as profiles
and context rules.

### 6. Validation is split into Structural and Execution phases

Validation is divided into two strict phases with a hard boundary:

**Structural Validation** operates only on serialized workflow data.
No I/O, no runtime state, no registry lookups. Can be determined
entirely from the Document. This phase runs during Sprint 1.

- Workflow ID is non-empty
- Workflow name is non-empty
- Workflow has at least one step
- All step action_ids are non-empty values
- No duplicate workflow IDs in Document
- WorkflowVersion is supported
- Structural correctness (all required fields present)

**Execution Validation** consults runtime services during invocation.
Runs after structural validation passes and before execution begins.
This phase runs during Sprint 2.

- Workflow exists in Document at invocation time
- Workflow is enabled
- All referenced action_ids exist in ActionRegistry
- All referenced actions are executable

**Boundary rule:** Structural validation operates only on serialized
workflow data and must not depend on runtime registries or execution
infrastructure. This permanently records the separation between the
domain model and the execution layer.

### 7. ExecutionTarget unifies dispatch

```rust
pub enum ExecutionTarget {
    Action(ActionId),
    Workflow(WorkflowId),
}
```

The agent handler resolves the opaque invocation target into an
`ExecutionTarget` and delegates to the appropriate executor. This
keeps a single dispatch path regardless of target type.

### 8. WorkflowStep is initially limited to ExecuteAction

```rust
// v0.6 — only this variant exists
pub struct WorkflowStep {
    pub action_id: ActionId,
    pub payload: Option<Value>,
}
```

Future versions may introduce Condition, Delay, Branch, or Loop
step types. The model is designed to accept these without modifying
existing validation architecture — new step types register their own
validators.

## Consequences

### Positive

- Preserves every v0.5 architectural boundary
- No protocol redesign required
- No Android coupling to workflow internals
- Desktop remains the sole authority for workflow definition, storage,
  validation, and execution
- Structural validation is independent of execution infrastructure,
  making it reusable for import, migration, and editor validation
- ActionId references are resilient to renaming
- WorkflowVersion supports future migration without format changes
- ExecutionTarget provides a stable extension point for future
  execution types (macros, scripts, triggers)

### Negative

- Workflow steps are limited to sequential execution in v0.6
- No conditional logic, loops, or parallel execution until v0.7+
- Workflow editing requires Desktop access (no mobile editing)

### Risks

- Long-running workflows (many steps × 5s timeout each) could block
  the runtime. Mitigated by per-step timeout and spawn_blocking.
- If workflows reference other workflows in the future, cycle detection
  becomes necessary. Explicitly out of scope for v0.6.

## Compliance

- Workflow definitions MUST be Desktop-authoritative (ADR-001)
- Workflow mutations MUST use the Command/Reducer pattern (ADR-008)
- Workflow steps MUST reference actions by ActionId, not name
- WorkflowVersion MUST be present in all stored workflows
- Android MUST NOT receive workflow definitions, step sequences, or
  execution plans
- Structural validation MUST NOT depend on runtime registries or
  execution infrastructure
- Execution validation MUST occur after structural validation passes
  and before execution begins
- All referenced actions MUST exist in ActionRegistry at invocation time
