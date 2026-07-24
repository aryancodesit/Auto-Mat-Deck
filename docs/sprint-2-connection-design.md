# Sprint 2 Design: SessionManager + ConnectionStateMachine

**Target:** v0.8 Sprint 2
**Depends on:** Sprint 1 (TrustStore, Discovery, PairingManager, DeviceIdentity)
**Commit:** Pending

## 1. Objective

Establish the connection lifecycle layer that sits between pairing
(discovery + trust establishment) and the application protocol
(projection, control, triggers). Sprint 2 delivers:

- **SessionManager** — owns the TCP connection to Desktop, manages
  identify handshake, routes incoming messages to the right handler
- **ConnectionStateMachine** — formalizes connection state transitions,
  prevents illegal state changes, emits state events for the UI

## 2. Architecture

```
Application Layer (Sprint 3+)
        │
        ▼
┌─────────────────────┐
│   SessionManager     │  ← owns connection lifecycle
│   (Kotlin)           │
└─────────┬───────────┘
          │
┌─────────▼───────────┐
│ ConnectionState      │  ← state machine + transitions
│ StateMachine         │
└─────────┬───────────┘
          │
┌─────────▼───────────┐
│   TCP Socket         │  ← java.net.Socket
│   (raw I/O)          │
└─────────────────────┘
          │
          ▼
    Desktop Agent
```

SessionManager owns both the TCP socket and the ConnectionStateMachine.
The state machine validates transitions and emits events — it owns no I/O.

```
SessionManager
    ├── owns Socket (java.net.Socket)
    └── owns ConnectionStateMachine (pure state)
```

## 3. ConnectionStateMachine

### States

```
                    ┌──────────┐
                    │ Disconn- │
                    │ ected    │
                    └────┬─────┘
                         │ connect()
                         ▼
                    ┌──────────┐
              ┌────►│ Connecting│
              │     └────┬─────┘
              │          │ socket connected
              │          ▼
              │     ┌──────────┐
              │     │ Identify-│
              │     │ ing      │
              │     └────┬─────┘
              │          │ identify sent + ack received
              │          ▼
              │     ┌──────────┐
              │     │ Connected │ ◄─── steady state
              │     └────┬─────┘
              │          │ error / timeout / remote close
              │          ▼
              │     ┌──────────┐
              │     │ Reconnect-│
              │     │ ing      │
              │     └────┬─────┘
              │          │ max retries exceeded
              │          ▼
              │     ┌──────────┐
              └─────│ Failed    │
                    └──────────┘
```

### Transitions

| From | To | Trigger |
|------|----|---------|
| Disconnected | Connecting | `connect()` called |
| Connecting | Identifying | TCP socket connected |
| Connecting | Disconnected | Connection refused/timeout |
| Identifying | Connected | `identify` ack received |
| Identifying | Disconnected | Identify timeout or rejected |
| Connected | Disconnected | Error, timeout, or remote close |
| Connected | Reconnecting | Connection lost (not explicit close) |
| Reconnecting | Connecting | Backoff timer expires |
| Reconnecting | Failed | Max retries exceeded |
| Failed | Disconnected | User-initiated reset |

### Implementation

```kotlin
enum class ConnectionState {
    Disconnected,
    Connecting,
    Identifying,
    Connected,
    Reconnecting,
    Failed
}

class ConnectionStateMachine(
    private val onStateChanged: (ConnectionState) -> Unit
) {
    private var state = ConnectionState.Disconnected

    fun getState(): ConnectionState = state

    fun transition(newState: ConnectionState): Boolean {
        if (!isAllowed(state, newState)) {
            Log.w(TAG, "Illegal transition: $state -> $newState")
            return false
        }
        Log.d(TAG, "State: $state -> $newState")
        state = newState
        onStateChanged(newState)
        return true
    }

    companion object {
        private val ALLOWED = mapOf(
            Disconnected to setOf(Connecting),
            Connecting to setOf(Identifying, Disconnected),
            Identifying to setOf(Connected, Disconnected),
            Connected to setOf(Disconnected, Reconnecting),
            Reconnecting to setOf(Connecting, Failed),
            Failed to setOf(Disconnected)
        )

        private fun isAllowed(from: ConnectionState, to: ConnectionState): Boolean {
            return from in ALLOWED && to in ALLOWED[from]!!
        }
    }
}
```

## 4. SessionManager

### Responsibilities

1. **Connect** — after pairing completes, establish TCP connection to
   Desktop using address from DiscoveryManager or DiscoveryCache
2. **Identify** — send `identify` message with DeviceIdentity, wait for
   ack
3. **Route** — dispatch incoming messages to registered handlers
4. **Disconnect** — clean shutdown, notify ConnectionStateMachine
5. **Provide** — expose `send(message)` for outbound messages

### API Surface

```kotlin
class SessionManager(
    private val context: Context,
    private val identity: DeviceIdentity,
    private val trustStore: TrustedDeviceStore
) {
    // Lifecycle
    fun connect(address: String, port: Int = 9551)
    fun disconnect()
    fun reset()  // clear session, go to Failed -> Disconnected

    // Message I/O
    fun send(message: String): Boolean
    fun onMessage(handler: (String) -> Unit)

    // State
    fun getState(): ConnectionState
    fun onStateChanged(handler: (ConnectionState) -> Unit)

    // Session info
    fun getDeviceId(): String  // from identity
    fun isConnected(): Boolean
}
```

### Identify Handshake

After TCP connect, SessionManager sends:

```json
{
    "type": "identify",
    "device_id": "550e8400-...",
    "device_name": "Android-Pixel 7"
}
```

Desktop responds with either:

```json
{"type": "trusted", "device_id": "550e8400-..."}
```
or:
```json
{"type": "untrusted", "message": "Device not paired."}
```

If `untrusted`, session is rejected and state goes to Disconnected.
If no response within 5 seconds, timeout -> Disconnected.

### Integration with Sprint 1

| Sprint 1 Module | Sprint 2 Usage |
|-----------------|----------------|
| DeviceIdentity | `identity.deviceId` for identify message |
| TrustStore (Android) | Verify device is trusted before connecting |
| DiscoveryManager | Get address for connect (or use cache) |
| PairingManager | After paired -> SessionManager.connect() |

### Thread Model

- Socket I/O runs on `Dispatchers.IO`
- State machine updates happen on `Dispatchers.Main`
- Message handlers are invoked on the socket reader thread
- `send()` is synchronous (caller blocks on IO dispatcher)

## 5. File Changes

### New Files

| File | Lines (est.) | Purpose |
|------|-------------|---------|
| `apps/mobile/.../connection/ConnectionState.kt` | ~50 | State enum + transition table |
| `apps/mobile/.../connection/ConnectionStateMachine.kt` | ~60 | State machine logic |
| `apps/mobile/.../connection/SessionManager.kt` | ~150 | Connection lifecycle |

### Modified Files

| File | Change |
|------|--------|
| `PairingManager.kt` | After `Paired` state, call `sessionManager.connect()` |
| `DiscoveryManager.kt` | No changes — SessionManager reads cache directly |

## 6. Test Plan

### ConnectionStateMachine Tests

1. Initial state is Disconnected
2. connect() -> Connecting
3. socket connected -> Identifying
4. identify ack -> Connected
5. error while Connected -> Disconnected
6. connection lost -> Reconnecting
7. backoff expires -> Connecting
8. max retries -> Failed
9. reset() from Failed -> Disconnected
10. Illegal transition rejected (e.g., Connected -> Connecting)

### SessionManager Tests

1. connect() transitions to Connecting
2. identify message format is correct
3. identify ack accepted -> Connected
4. identify ack rejected -> Disconnected
5. identify timeout -> Disconnected
6. send() while connected -> writes to socket
7. send() while disconnected -> returns false
8. incoming message dispatched to handler
9. disconnect() cleans up socket
10. reset() clears session state

### Integration Test

1. Pairing complete -> SessionManager.connect() -> Connected
2. Disconnect -> state -> Disconnected
3. Reconnect after disconnect -> Reconnecting -> Connected

## 7. Deferred to Sprint 3

- HeartbeatService (keepalive pings)
- ReconnectManager (exponential backoff, retry policy)
- RecoveryManager (session restore after reconnect)

Sprint 2 uses a simple fixed reconnect (3 retries, 5s delay).
Heartbeat and exponential backoff are Sprint 3 concerns.

## 8. Risks

- **Socket lifecycle** — Android may kill background sockets. Mitigation:
  Foreground service for persistent connection (Sprint 3 concern).
- **Thread safety** — socket read/write from different coroutines.
  Mitigation: single socket reader coroutine, synchronized send.
- **Desktop side** — Desktop needs to handle `identify` message. This
  already exists in the protocol but may need desktop-side wiring.
