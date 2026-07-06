# Core Domain Model

> Canonical definitions for the core concepts of Auto-Mat-Deck.

---

## Action

An **Action** is a single atomic operation that the desktop agent can execute.

- **Installed by**: Plugins
- **Configured by**: Mobile (via capability-driven UI)
- **Executed by**: Desktop agent

Examples: `OpenApplication`, `SendKeystroke`, `SetVolume`, `RunScript`, `HttpRequest`.

### Action Definition

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique action identifier within a plugin |
| `name` | string | Human-readable name |
| `description` | string | Short description of what the action does |
| `pluginId` | string | Plugin that provides this action |
| `parameters` | Parameter[] | Input parameters the action accepts |
| `output` | OutputType | Type of output the action produces |

---

## Macro

A **Macro** is a sequence of actions executed in order.

- **Created by**: User (on mobile)
- **Stored in**: A deck
- **Executed by**: Desktop agent

Macros can include:
- Linear execution (actions run one after another)
- Simple branching (if/else based on action output)
- Configurable delays between steps

### Macro Definition

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier |
| `name` | string | Human-readable name |
| `steps` | Step[] | Ordered list of actions with parameters |
| `errorHandling` | enum | `stop`, `continue`, `retry` |

---

## Workflow

A **Workflow** is a stateful, multi-step process triggered by events.

- More complex than a macro.
- Can include conditions, loops, state, and parallel branches.
- Spans multiple trigger-action pairs.
- Has a lifecycle (idle, running, paused, completed, failed).

### Workflow Definition

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier |
| `name` | string | Human-readable name |
| `trigger` | TriggerRef | Trigger that starts this workflow |
| `states` | State[] | Individual states in the workflow |
| `transitions` | Transition[] | Rules for moving between states |
| `context` | ContextRef | Context bindings this workflow observes |

---

## Trigger

A **Trigger** is a condition that, when met, causes the desktop agent to execute an action, macro, or workflow.

See [Triggers.md](./Triggers.md) for the full trigger taxonomy.

### Trigger Definition

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier |
| `type` | enum | `manual`, `time`, `file`, `context`, `application`, `webhook`, `custom` |
| `condition` | object | Type-specific condition parameters |
| `boundAction` | ActionRef | Action to execute when triggered |
| `enabled` | boolean | Whether the trigger is active |
| `cooldown` | duration | Minimum interval between firings |

---

## Profile

A **Profile** is a named, portable set of user configuration that can be activated and synced to the desktop agent.

See [Profiles.md](./Profiles.md) for the full profile model.

### Profile Definition

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier |
| `name` | string | User-defined name |
| `desktopPairing` | DeviceRef | Paired desktop device |
| `decks` | Deck[] | Collection of decks in this profile |
| `active` | boolean | Whether this is the active profile |
| `createdAt` | timestamp | Creation timestamp |
| `updatedAt` | timestamp | Last modification timestamp |

---

## Context

**Context** is a snapshot of the desktop environment at a point in time.

- **Monitored by**: Desktop agent (and plugins)
- **Consumed by**: Triggers and workflows
- **Reported to**: Mobile (real-time events)

### Context Categories

| Category | Examples |
|----------|---------|
| System | CPU, memory, disk, network, battery |
| Process | Running processes, foreground window, active application |
| File | Recently modified files, watched directories |
| Device | Connected peripherals, audio devices, displays |
| Custom | Plugin-defined context sources |
