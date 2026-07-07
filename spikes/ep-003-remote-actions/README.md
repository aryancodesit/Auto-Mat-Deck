# EP-003 — Remote Actions

**Status**: ✅ Certified

**Goal**: Prove a trusted phone can ask the desktop to perform an action.

## Scope

Exactly 5 actions. Nothing else.

| Action | JSON | Desktop Behavior |
|--------|------|------------------|
| Launch Application | `{"action":"launch","payload":{"app":"chrome"}}` | `cmd /c start "" <app>` |
| Open URL | `{"action":"open_url","payload":{"url":"https://..."}}` | `cmd /c start "" <url>` |
| Open File | `{"action":"open_file","payload":{"path":"C:\\..."}}` | `cmd /c start "" <path>` |
| Lock WorkStation | `{"action":"lock","payload":{}}` | `LockWorkStation()` |
| Desktop Notification | `{"action":"notify","payload":{"title":"...","body":"..."}}` | PowerShell WinRT toast |

## Protocol

Every action request:

```json
{
  "type": "action",
  "request_id": "unique-id",
  "action": "launch",
  "payload": { "app": "chrome" }
}
```

Every response:

```json
{
  "type": "action_result",
  "request_id": "unique-id",
  "success": true,
  "data": { "pid": 1234 }
}
```

On failure:

```json
{
  "type": "action_result",
  "request_id": "unique-id",
  "success": false,
  "error": "Missing 'app' in payload"
}
```

## Architecture

```
desktop/
  main.rs
  actions.rs   ← Action trait, ActionRegistry, 5 action impls
  discovery.rs
```

Execution flow:

```
WebSocket → Parse → Validate trusted → Route → Action::execute() → Respond
```

`ActionRegistry` uses a `HashMap<&str, Box<dyn Action>>` with a `LazyLock` static.
Each action implements `Action: Send + Sync` with an `execute(&self, &Value) -> Result<Value, ActionError>`.

## Excluded (future)

- Keyboard / mouse simulation
- Shell / PowerShell execution
- Arbitrary process execution
- Macros, plugins, scripting
- User-defined actions
- Profiles, pages, themes

## Completion Criteria

| Test | Result |
|------|--------|
| `launch chrome` → chrome.exe opens | ✅ |
| `open_url github.com` → browser opens | ✅ |
| `open_file calc.exe` → Calculator opens | ✅ |
| `lock` → WorkStation locks | ✅ |
| `notify` → Windows toast appears | ✅ |
