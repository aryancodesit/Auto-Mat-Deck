package com.automatdeck.app.connection

import android.util.Log
import com.automatdeck.app.discovery.DiscoveryCache
import com.automatdeck.app.identity.DeviceIdentity
import com.automatdeck.app.pairing.TrustedDeviceStore
import kotlinx.coroutines.*
import org.json.JSONObject
import java.io.BufferedReader
import java.io.InputStreamReader
import java.io.OutputStreamWriter
import java.net.Socket

class SessionManager(
    private val identity: DeviceIdentity,
    private val trustStore: TrustedDeviceStore,
    private val discoveryCache: DiscoveryCache,
    private val mainScope: CoroutineScope = CoroutineScope(Dispatchers.Main + SupervisorJob())
) {
    private val stateMachine = ConnectionStateMachine { state ->
        onStateChanged?.invoke(state)
    }

    private var socket: Socket? = null
    private var reader: BufferedReader? = null
    private var writer: OutputStreamWriter? = null
    private var readerJob: Job? = null

    private var onStateChanged: ((ConnectionState) -> Unit)? = null
    private var onMessage: ((String) -> Unit)? = null

    fun getState(): ConnectionState = stateMachine.getState()

    fun isConnected(): Boolean = stateMachine.getState() == ConnectionState.Connected

    fun onStateChanged(handler: (ConnectionState) -> Unit) {
        onStateChanged = handler
    }

    fun onMessage(handler: (String) -> Unit) {
        onMessage = handler
    }

    fun connect() {
        val currentState = stateMachine.getState()
        if (currentState != ConnectionState.Disconnected &&
            currentState != ConnectionState.Failed
        ) {
            Log.w(TAG, "Cannot connect from state $currentState")
            return
        }

        transitionTo(ConnectionState.Connecting)

        mainScope.launch(Dispatchers.IO) {
            try {
                // Refuse connection when the desktop is no longer trusted
                val trustedDevice = trustStore.get()
                if (trustedDevice == null) {
                    Log.w(TAG, "Connection refused: No trusted device configured")
                    transitionTo(ConnectionState.Failed)
                    return@launch
                }

                // Resolve host/port: check if DiscoveryCache has a more recent entry for the same deviceId
                val lastKnown = discoveryCache.getLastKnown()
                val host = if (lastKnown != null && lastKnown.deviceId == trustedDevice.deviceId) {
                    lastKnown.host
                } else {
                    trustedDevice.host
                }
                val port = if (lastKnown != null && lastKnown.deviceId == trustedDevice.deviceId) {
                    lastKnown.port
                } else {
                    trustedDevice.port
                }

                Log.i(TAG, "Connecting to trusted desktop at $host:$port")
                val sock = Socket(host, port)
                sock.soTimeout = READ_TIMEOUT_MS.toInt()
                socket = sock
                reader = BufferedReader(InputStreamReader(sock.getInputStream()))
                writer = OutputStreamWriter(sock.getOutputStream())

                transitionTo(ConnectionState.Identifying)
                startReading()
                startIdentifyTimeout()
            } catch (e: Exception) {
                Log.e(TAG, "Connection failed: ${e.message}")
                cleanup()
                transitionTo(ConnectionState.Disconnected)
            }
        }
    }

    fun disconnect() {
        mainScope.launch {
            cleanup()
            stateMachine.transition(ConnectionState.Disconnected)
        }
    }

    fun reset() {
        mainScope.launch {
            cleanup()
            stateMachine.reset()
        }
    }

    fun send(message: String): Boolean {
        if (!isConnected()) return false
        val w = writer ?: return false
        return try {
            synchronized(w) {
                w.write(message + "\n")
                w.flush()
            }
            true
        } catch (e: Exception) {
            Log.e(TAG, "Send failed: ${e.message}")
            false
        }
    }

    private fun sendIdentifyInternal(): Boolean {
        val identify = JSONObject().apply {
            put("type", "identify")
            put("device_id", identity.deviceId)
            put("device_name", identity.deviceName)
        }
        val msg = identify.toString()
        val w = writer ?: return false
        return try {
            synchronized(w) {
                w.write(msg + "\n")
                w.flush()
            }
            Log.d(TAG, "Identify sent: ${identity.deviceId}")
            true
        } catch (e: Exception) {
            Log.e(TAG, "Failed to send identify: ${e.message}")
            false
        }
    }

    private fun startReading() {
        readerJob = mainScope.launch(Dispatchers.IO) {
            try {
                // Send identify immediately as soon as reader starts
                if (!sendIdentifyInternal()) {
                    onConnectionClosed()
                    return@launch
                }

                while (isActive) {
                    val line = reader?.readLine() ?: break
                    handleIncomingLine(line)
                }
            } catch (e: Exception) {
                if (isActive) {
                    Log.w(TAG, "Read loop ended: ${e.message}")
                }
            }
            onConnectionClosed()
        }
    }

    private fun handleIncomingLine(line: String) {
        val currentState = stateMachine.getState()
        if (currentState == ConnectionState.Identifying) {
            handleIdentifyResponse(line)
        } else if (currentState == ConnectionState.Connected) {
            onMessage?.invoke(line)
        } else {
            Log.w(TAG, "Received message in unexpected state $currentState: $line")
        }
    }

    private fun handleIdentifyResponse(response: String) {
        val json = try { JSONObject(response) } catch (e: Exception) {
            Log.w(TAG, "Invalid identify response: $response")
            transitionTo(ConnectionState.Disconnected)
            cleanup()
            return
        }

        when (json.optString("type")) {
            "trusted" -> {
                Log.i(TAG, "Identify accepted, session established")
                transitionTo(ConnectionState.Connected)
            }
            "untrusted" -> {
                Log.w(TAG, "Identify rejected: ${json.optString("message")}")
                transitionTo(ConnectionState.Disconnected)
                cleanup()
            }
            else -> {
                Log.w(TAG, "Unknown identify response type: ${json.optString("type")}")
                transitionTo(ConnectionState.Disconnected)
                cleanup()
            }
        }
    }

    private fun startIdentifyTimeout() {
        mainScope.launch {
            delay(IDENTIFY_TIMEOUT_MS)
            if (stateMachine.getState() == ConnectionState.Identifying) {
                Log.w(TAG, "Identify handshake timed out")
                transitionTo(ConnectionState.Disconnected)
                cleanup()
            }
        }
    }

    private fun onConnectionClosed() {
        mainScope.launch {
            cleanup()
            if (stateMachine.getState() == ConnectionState.Connected ||
                stateMachine.getState() == ConnectionState.Identifying
            ) {
                stateMachine.transition(ConnectionState.Disconnected)
            }
        }
    }

    private fun transitionTo(newState: ConnectionState) {
        mainScope.launch {
            stateMachine.transition(newState)
        }
    }

    private fun cleanup() {
        readerJob?.cancel()
        readerJob = null
        try { reader?.close() } catch (_: Exception) {}
        try { writer?.close() } catch (_: Exception) {}
        try { socket?.close() } catch (_: Exception) {}
        reader = null
        writer = null
        socket = null
    }

    companion object {
        private const val TAG = "SessionManager"
        private const val READ_TIMEOUT_MS = 30_000L
        private const val IDENTIFY_TIMEOUT_MS = 5000L
    }
}
