package com.automatdeck.app.connection

import org.junit.Assert.*
import org.junit.Before
import org.junit.Test

class ConnectionStateMachineTest {

    private lateinit var machine: ConnectionStateMachine
    private val states = mutableListOf<ConnectionState>()

    @Before
    fun setup() {
        states.clear()
        machine = ConnectionStateMachine { states.add(it) }
    }

    @Test
    fun initialStateIsDisconnected() {
        assertEquals(ConnectionState.Disconnected, machine.getState())
    }

    @Test
    fun connectTransitionsToConnecting() {
        assertTrue(machine.transition(ConnectionState.Connecting))
        assertEquals(ConnectionState.Connecting, machine.getState())
        assertEquals(listOf(ConnectionState.Connecting), states)
    }

    @Test
    fun connectingToIdentifying() {
        machine.transition(ConnectionState.Connecting)
        assertTrue(machine.transition(ConnectionState.Identifying))
        assertEquals(ConnectionState.Identifying, machine.getState())
    }

    @Test
    fun connectingToDisconnectedOnFailure() {
        machine.transition(ConnectionState.Connecting)
        assertTrue(machine.transition(ConnectionState.Disconnected))
        assertEquals(ConnectionState.Disconnected, machine.getState())
    }

    @Test
    fun identifyingToConnected() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        assertTrue(machine.transition(ConnectionState.Connected))
        assertEquals(ConnectionState.Connected, machine.getState())
    }

    @Test
    fun identifyingToDisconnectedOnReject() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        assertTrue(machine.transition(ConnectionState.Disconnected))
        assertEquals(ConnectionState.Disconnected, machine.getState())
    }

    @Test
    fun connectedToDisconnectedOnError() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        machine.transition(ConnectionState.Connected)
        assertTrue(machine.transition(ConnectionState.Disconnected))
        assertEquals(ConnectionState.Disconnected, machine.getState())
    }

    @Test
    fun connectedToReconnectingOnLostConnection() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        machine.transition(ConnectionState.Connected)
        assertTrue(machine.transition(ConnectionState.Reconnecting))
        assertEquals(ConnectionState.Reconnecting, machine.getState())
    }

    @Test
    fun reconnectingToConnecting() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        machine.transition(ConnectionState.Connected)
        machine.transition(ConnectionState.Reconnecting)
        assertTrue(machine.transition(ConnectionState.Connecting))
        assertEquals(ConnectionState.Connecting, machine.getState())
    }

    @Test
    fun reconnectingToFailedAfterMaxRetries() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        machine.transition(ConnectionState.Connected)
        machine.transition(ConnectionState.Reconnecting)
        assertTrue(machine.transition(ConnectionState.Failed))
        assertEquals(ConnectionState.Failed, machine.getState())
    }

    @Test
    fun failedToDisconnectedOnReset() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        machine.transition(ConnectionState.Connected)
        machine.transition(ConnectionState.Reconnecting)
        machine.transition(ConnectionState.Failed)
        assertTrue(machine.transition(ConnectionState.Disconnected))
        assertEquals(ConnectionState.Disconnected, machine.getState())
    }

    @Test
    fun illegalTransitionRejected() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        machine.transition(ConnectionState.Connected)
        // Connected -> Connecting is illegal
        assertFalse(machine.transition(ConnectionState.Connecting))
        assertEquals(ConnectionState.Connected, machine.getState())
        assertEquals(emptyList<ConnectionState>(), states)
    }

    @Test
    fun disconnectedToIdentifyingRejected() {
        assertFalse(machine.transition(ConnectionState.Identifying))
        assertEquals(ConnectionState.Disconnected, machine.getState())
    }

    @Test
    fun disconnectedToConnectedRejected() {
        assertFalse(machine.transition(ConnectionState.Connected))
        assertEquals(ConnectionState.Disconnected, machine.getState())
    }

    @Test
    fun disconnectedToReconnectingRejected() {
        assertFalse(machine.transition(ConnectionState.Reconnecting))
        assertEquals(ConnectionState.Disconnected, machine.getState())
    }

    @Test
    fun disconnectedToFailedRejected() {
        assertFalse(machine.transition(ConnectionState.Failed))
        assertEquals(ConnectionState.Disconnected, machine.getState())
    }

    @Test
    fun connectingToConnectedRejected() {
        machine.transition(ConnectionState.Connecting)
        assertFalse(machine.transition(ConnectionState.Connected))
        assertEquals(ConnectionState.Connecting, machine.getState())
    }

    @Test
    fun connectingToReconnectingRejected() {
        machine.transition(ConnectionState.Connecting)
        assertFalse(machine.transition(ConnectionState.Reconnecting))
        assertEquals(ConnectionState.Connecting, machine.getState())
    }

    @Test
    fun connectingToFailedRejected() {
        machine.transition(ConnectionState.Connecting)
        assertFalse(machine.transition(ConnectionState.Failed))
        assertEquals(ConnectionState.Connecting, machine.getState())
    }

    @Test
    fun identifyingToConnectingRejected() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        assertFalse(machine.transition(ConnectionState.Connecting))
        assertEquals(ConnectionState.Identifying, machine.getState())
    }

    @Test
    fun identifyingToReconnectingRejected() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        assertFalse(machine.transition(ConnectionState.Reconnecting))
        assertEquals(ConnectionState.Identifying, machine.getState())
    }

    @Test
    fun identifyingToFailedRejected() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        assertFalse(machine.transition(ConnectionState.Failed))
        assertEquals(ConnectionState.Identifying, machine.getState())
    }

    @Test
    fun connectedToIdentifyingRejected() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        machine.transition(ConnectionState.Connected)
        assertFalse(machine.transition(ConnectionState.Identifying))
        assertEquals(ConnectionState.Connected, machine.getState())
    }

    @Test
    fun connectedToFailedRejected() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        machine.transition(ConnectionState.Connected)
        assertFalse(machine.transition(ConnectionState.Failed))
        assertEquals(ConnectionState.Connected, machine.getState())
    }

    @Test
    fun reconnectingToIdentifyingRejected() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        machine.transition(ConnectionState.Connected)
        machine.transition(ConnectionState.Reconnecting)
        assertFalse(machine.transition(ConnectionState.Identifying))
        assertEquals(ConnectionState.Reconnecting, machine.getState())
    }

    @Test
    fun reconnectingToConnectedRejected() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        machine.transition(ConnectionState.Connected)
        machine.transition(ConnectionState.Reconnecting)
        assertFalse(machine.transition(ConnectionState.Connected))
        assertEquals(ConnectionState.Reconnecting, machine.getState())
    }

    @Test
    fun reconnectingToDisconnectedRejected() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        machine.transition(ConnectionState.Connected)
        machine.transition(ConnectionState.Reconnecting)
        assertFalse(machine.transition(ConnectionState.Disconnected))
        assertEquals(ConnectionState.Reconnecting, machine.getState())
    }

    @Test
    fun failedToConnectingRejected() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        machine.transition(ConnectionState.Connected)
        machine.transition(ConnectionState.Reconnecting)
        machine.transition(ConnectionState.Failed)
        assertFalse(machine.transition(ConnectionState.Connecting))
        assertEquals(ConnectionState.Failed, machine.getState())
    }

    @Test
    fun failedToIdentifyingRejected() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        machine.transition(ConnectionState.Connected)
        machine.transition(ConnectionState.Reconnecting)
        machine.transition(ConnectionState.Failed)
        assertFalse(machine.transition(ConnectionState.Identifying))
        assertEquals(ConnectionState.Failed, machine.getState())
    }

    @Test
    fun failedToConnectedRejected() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        machine.transition(ConnectionState.Connected)
        machine.transition(ConnectionState.Reconnecting)
        machine.transition(ConnectionState.Failed)
        assertFalse(machine.transition(ConnectionState.Connected))
        assertEquals(ConnectionState.Failed, machine.getState())
    }

    @Test
    fun failedToReconnectingRejected() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        machine.transition(ConnectionState.Connected)
        machine.transition(ConnectionState.Reconnecting)
        machine.transition(ConnectionState.Failed)
        assertFalse(machine.transition(ConnectionState.Reconnecting))
        assertEquals(ConnectionState.Failed, machine.getState())
    }

    @Test
    fun resetGoesToDisconnected() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        machine.transition(ConnectionState.Connected)
        machine.reset()
        assertEquals(ConnectionState.Disconnected, machine.getState())
        assertEquals(listOf(ConnectionState.Disconnected), states)
    }

    @Test
    fun fullHappyPath() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        machine.transition(ConnectionState.Connected)
        machine.transition(ConnectionState.Disconnected)
        assertEquals(ConnectionState.Disconnected, machine.getState())
    }

    @Test
    fun reconnectCycle() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        machine.transition(ConnectionState.Connected)
        machine.transition(ConnectionState.Reconnecting)
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        machine.transition(ConnectionState.Connected)
        assertEquals(ConnectionState.Connected, machine.getState())
    }

    @Test
    fun callbackReceivesAllTransitions() {
        machine.transition(ConnectionState.Connecting)
        machine.transition(ConnectionState.Identifying)
        machine.transition(ConnectionState.Connected)
        assertEquals(
            listOf(
                ConnectionState.Connecting,
                ConnectionState.Identifying,
                ConnectionState.Connected
            ),
            states
        )
    }

    @Test
    fun isAllowedStaticMethod() {
        assertTrue(ConnectionStateMachine.isAllowed(ConnectionState.Disconnected, ConnectionState.Connecting))
        assertFalse(ConnectionStateMachine.isAllowed(ConnectionState.Disconnected, ConnectionState.Connected))
        assertTrue(ConnectionStateMachine.isAllowed(ConnectionState.Connected, ConnectionState.Reconnecting))
        assertFalse(ConnectionStateMachine.isAllowed(ConnectionState.Connected, ConnectionState.Connecting))
    }
}
