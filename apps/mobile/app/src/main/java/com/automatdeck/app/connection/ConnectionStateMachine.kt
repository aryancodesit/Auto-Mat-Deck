package com.automatdeck.app.connection

import android.util.Log

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

    fun reset() {
        state = ConnectionState.Disconnected
        onStateChanged(state)
    }

    companion object {
        private const val TAG = "ConnectionStateMachine"

        private val ALLOWED = mapOf(
            ConnectionState.Disconnected to setOf(ConnectionState.Connecting),
            ConnectionState.Connecting to setOf(ConnectionState.Identifying, ConnectionState.Disconnected),
            ConnectionState.Identifying to setOf(ConnectionState.Connected, ConnectionState.Disconnected),
            ConnectionState.Connected to setOf(ConnectionState.Disconnected, ConnectionState.Reconnecting),
            ConnectionState.Reconnecting to setOf(ConnectionState.Connecting, ConnectionState.Failed),
            ConnectionState.Failed to setOf(ConnectionState.Disconnected)
        )

        fun isAllowed(from: ConnectionState, to: ConnectionState): Boolean {
            return ALLOWED[from]?.contains(to) == true
        }
    }
}
