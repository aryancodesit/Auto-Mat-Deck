# Plugin System

> Desktop agent extensibility via plugins.

## Design

- The desktop agent defines a stable plugin interface.
- Plugins are self-contained units that can provide actions, triggers, or context monitors.
- The agent discovers and loads plugins at startup.

## Plugin Interface

TBD — likely one of:
- Shared library (`.dll` / `.so` / `.dylib`)
- Script-based (Lua, Python, JavaScript)
- Subprocess (language-agnostic via stdin/stdout)

## Capabilities

Each plugin advertises:
- **Actions** — executable operations
- **Triggers** — conditions that can fire
- **Context** — system state it can monitor
- **Configuration** — user-settable parameters

## Lifecycle

1. **Discovery** — Agent scans plugin directory
2. **Loading** — Plugin is loaded and capability report generated
3. **Registration** — Capabilities are registered in the agent's capability store
4. **Operation** — Plugin runs in response to commands or events
5. **Unloading** — Plugin is stopped and resources released
