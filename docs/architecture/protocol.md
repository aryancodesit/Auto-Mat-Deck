# Wire Protocol

## Status: Draft v0.1.0-draft

The protocol is currently **implemented ad-hoc in the desktop codebase**
(`apps/desktop/src/agent.rs`). A formal shared definition will be created under
`shared/protocol/` as the system stabilizes.

## Design

- **Transport**: WebSocket (TCP, plaintext in v1).
- **Message format**: JSON, one frame per message.
- **Direction**: Bidirectional after connection is established.

## Message catalog

| Message | Sender | Receiver | Purpose |
|---------|--------|----------|---------|
| `identify` | Mobile | Desktop | Identify the connecting device before trust evaluation |
| `trusted` | Desktop | Mobile | Confirm device is already paired and trusted |
| `untrusted` | Desktop | Mobile | Indicate device is not paired |
| `pair_request` | Mobile | Desktop | Request to begin trust establishment |
| `pair_accepted` | Desktop | Mobile | Confirm pairing was approved by user |
| `pair_rejected` | Desktop | Mobile | Indicate pairing was declined or timed out |
| `action` | Mobile | Desktop | Request execution of a remote action |
| `action_result` | Desktop | Mobile | Result of action execution |
| `ping` | Mobile | Desktop | Keepalive probe |
| `pong` | Desktop | Mobile | Keepalive response |
| `error` | Desktop | Mobile | Error response to invalid messages |

## Implementation status

| Message | Status |
|---------|--------|
| `identify` | ✅ Implemented |
| `trusted` | ✅ Implemented |
| `untrusted` | ✅ Implemented |
| `pair_request` | ✅ Implemented |
| `pair_accepted` | ✅ Implemented |
| `pair_rejected` | ✅ Implemented |
| `action` | ✅ Implemented |
| `action_result` | ✅ Implemented |
| `ping` | ✅ Implemented |
| `pong` | ✅ Implemented |
| `error` | ✅ Implemented |
| `pair_challenge` | ⏳ Planned (OTP enhancement, ADR-002) |
| `pair_verify` | ⏳ Planned (OTP enhancement, ADR-002) |

All payload examples below are **Draft v0.1.0-draft** and may change until
the wire format is frozen at v1.

### `identify`

- **Sender:** Mobile
- **Receiver:** Desktop
- **Purpose:** Identify the connecting device. Sent immediately after WebSocket
  open, before any other message.

```json
{ "type": "identify", "device_id": "android-RMX3392", "device_name": "Pixel 7" }
```

### `trusted`

- **Sender:** Desktop
- **Receiver:** Mobile
- **Purpose:** Confirm the identified device is already paired and trusted.
  Subsequent messages from this device are accepted immediately.

```json
{ "type": "trusted", "device_id": "amd-MY-DESKTOP" }
```

### `untrusted`

- **Sender:** Desktop
- **Receiver:** Mobile
- **Purpose:** Indicate the device is not paired. Mobile should follow up with
  a `pair_request` if the user wants to initiate pairing.

```json
{ "type": "untrusted", "message": "Device not paired. Send pair_request to initiate pairing." }
```

### `pair_request`

- **Sender:** Mobile
- **Receiver:** Desktop
- **Purpose:** Request to pair. Desktop prompts the user (via GUI or tray
  notification) to approve or decline.

```json
{ "type": "pair_request", "device_name": "My Phone" }
```

### `pair_accepted`

- **Sender:** Desktop
- **Receiver:** Mobile
- **Purpose:** Confirm the user approved the pairing. The device is now trusted.

```json
{ "type": "pair_accepted", "device_id": "amd-MY-DESKTOP" }
```

### `pair_rejected`

- **Sender:** Desktop
- **Receiver:** Mobile
- **Purpose:** Indicate the user declined the pairing, or the pairing timed out.

```json
{ "type": "pair_rejected", "device_id": "amd-MY-DESKTOP", "reason": "User declined or timeout" }
```

### `action`

- **Sender:** Mobile
- **Receiver:** Desktop
- **Purpose:** Request execution of a remote action. Only accepted from trusted
  (paired) devices.

```json
{ "type": "action", "action": "lock", "device_id": "amd-MY-DESKTOP", "request_id": "req-001" }
```

### `action_result`

- **Sender:** Desktop
- **Receiver:** Mobile
- **Purpose:** Result of an action execution. Includes success status and
  optional data.

```json
{ "type": "action_result", "request_id": "req-001", "success": true, "data": {} }
```

### `ping`

- **Sender:** Mobile
- **Receiver:** Desktop
- **Purpose:** Keepalive probe. Desktop disconnects clients that do not send
  periodic pings.

```json
{ "type": "ping" }
```

### `pong`

- **Sender:** Desktop
- **Receiver:** Mobile
- **Purpose:** Keepalive response. Echoes the original ping for correlation.

```json
{ "type": "pong", "echo": {}, "deviceId": "192.168.1.5:9742" }
```

### `error`

- **Sender:** Desktop
- **Receiver:** Mobile
- **Purpose:** Returned when a message is invalid, missing required fields, or
  sent in the wrong state (e.g., action from an unpaired device).

```json
{ "type": "error", "message": "Device not paired. Complete pairing first." }
```

## Sequence (current v0.x pairing flow)

```mermaid
sequenceDiagram
    participant M as Mobile
    participant D as Desktop

    M->>D: WebSocket connect
    M->>D: identify (device_id, device_name)

    alt Device already paired
        D-->>M: trusted
    else Device not paired
        D-->>M: untrusted
        M->>D: pair_request (device_name)
        Note over D: User approves/declines via tray or GUI
        alt Approved
            D-->>M: pair_accepted
        else Declined or timeout
            D-->>M: pair_rejected
        end
    end

    Note over M,D: Subsequent sessions skip pairing

    M->>D: action (action, request_id)
    D-->>M: action_result (success, data)

    M->>D: ping
    D-->>M: pong
```

## Protocol Roadmap

- TLS for transport security.
- Message versioning in the header.
- Protobuf or similar for schema enforcement.
