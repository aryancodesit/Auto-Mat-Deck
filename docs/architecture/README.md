# Architecture Documentation

| Document | Description |
|----------|-------------|
| [Overview](./Overview.md) | High-level system architecture and design philosophy |
| [Protocol](./Protocol.md) | Mobile ↔ Desktop communication protocol specification |
| [Discovery](./Discovery.md) | Device discovery on the local network |
| [Pairing & Authentication](./Pairing-Authentication.md) | Trust establishment and session management |
| [Synchronization](./Synchronization.md) | State ownership and sync between devices |
| [Security Model](./Security-Model.md) | Threat model, trust, and key management |
| [Core Domain Model](./Core-Domain-Model.md) | Canonical definitions for Action, Macro, Workflow, Trigger, Profile, Context |
| [Profiles](./Profiles.md) | Profile configuration and lifecycle |
| [Triggers](./Triggers.md) | Trigger types and execution model |
| [Plugin System](./Plugin-System.md) | Desktop agent plugin architecture |

## Validation

Architecture assumptions are validated experimentally. See [`spikes/`](../../spikes/) for disposable prototypes that test critical path decisions on real devices before production code is written.
