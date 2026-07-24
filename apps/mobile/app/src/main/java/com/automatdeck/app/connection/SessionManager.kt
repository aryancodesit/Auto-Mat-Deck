package com.automatdeck.app.connection

import android.util.Log
import com.automatdeck.app.identity.DeviceIdentity
import kotlinx.coroutines.*
import org.json.JSONObject
import java.io.BufferedReader
import java.io.InputStreamReader
import java.io.OutputStreamWriter
import java.net.Socket
import java.net.SocketTimeoutException

class SessionManager(
    private val identity: DeviceIdentity
) {
    private val stateMachine = ConnectionStateMachine { state ->
        onStateChanged?.invoke(state)
    }

    private var socket: Socket? = null
    private var reader: BufferedReader? = null
    private var writer: OutputStreamWriter? = null
    private var readerJob: Job? = null
    private var scope: CoroutineScope? = null

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

    fun connect(address: String, port: Int = DEFAULT_PORT, scope: CoroutineScope) {
        if (stateMachine.getState() != ConnectionState.Disconnected &&
            stateMachine.getState() != ConnectionState.Failed
        ) {
            Log.w(TAG, "Cannot connect from state ${stateMachine.getState()}")
            return
        }

        this.scope = scope
        stateMachine.transition(ConnectionState.Connecting)

        scope.launch(Dispatchers.IO) {
            try {
                val sock = Socket(address, port)
                sock.soTimeout = READ_TIMEOUT_MS.toInt()
                socket = sock
                reader = BufferedReader(InputStreamReader(sock.getInputStream()))
                writer = OutputStreamWriter(sock.getOutputStream())

                stateMachine.transition(ConnectionState.Identifying)
                sendIdentify()

                startReading()
            } catch (e: Exception) {
                Log.e(TAG, "Connection failed: ${e.message}")
                cleanup()
                stateMachine.transition(ConnectionState.Disconnected)
            }
        }
    }

    fun disconnect() {
        cleanup()
        stateMachine.transition(ConnectionState.Disconnected)
    }

    fun reset() {
        cleanup()
        stateMachine.reset()
    }

    fun send(message: String): Boolean {
        if (!isConnected()) return false
        return try {
            synchronized(writer!!) {
                writer!!.write(message + "\n")
                writer!!.flush()
            }
            true
        } catch (e: Exception) {
            Log.e(TAG, "Send failed: ${e.message}")
            false
        }
    }

    private fun sendIdentify() {
        val identify = JSONObject().apply {
            put("type", "identify")
            put("device_id", identity.deviceId)
            put("device_name", identity.deviceName)
        }
        val sent = send(identify.toString())
        if (!sent) {
            Log.e(TAG, "Failed to send identify")
            cleanup()
            stateMachine.transition(ConnectionState.Disconnected)
            return
        }
        Log.d(TAG, "Identify sent: ${identity.deviceId}")

        // Wait for response with timeout
        scope?.launch(Dispatchers.IO) {
            try {
                val response = reader?.readLine()
                if (response == null) {
                    Log.w(TAG, "Identify response: connection closed")
                    cleanup()
                    stateMachine.transition(ConnectionState.Disconnected)
                    return@launch
                }
                handleIdentifyResponse(response)
            } catch (e: SocketTimeoutException) {
                Log.w(TAG, "Identify timeout")
                cleanup()
                stateMachine.transition(ConnectionState.Disconnected)
            } catch (e: Exception) {
                Log.e(TAG, "Identify read error: ${e.message}")
                cleanup()
                stateMachine.transition(ConnectionState.Disconnected)
            }
        }
    }

    private fun handleIdentifyResponse(response: String) {
        val json = try { JSONObject(response) } catch (e: Exception) {
            Log.w(TAG, "Invalid identify response: $response")
            cleanup()
            stateMachine.transition(ConnectionState.Disconnected)
            return
        }

        when (json.optString("type")) {
            "trusted" -> {
                Log.i(TAG, "Identify accepted, session established")
                stateMachine.transition(ConnectionState.Connected)
            }
            "untrusted" -> {
                Log.w(TAG, "Identify rejected: ${json.optString("message")}")
                cleanup()
                stateMachine.transition(ConnectionState.Disconnected)
            }
            else -> {
                Log.w(TAG, "Unknown identify response type: ${json.optString("type")}")
                cleanup()
                stateMachine.transition(ConnectionState.Disconnected)
            }
        }
    }

    private fun startReading() {
        readerJob = scope?.launch(Dispatchers.IO) {
            try {
                while (isActive) {
                    val line = reader?.readLine() ?: break
                    onMessage?.invoke(line)
                }
            } catch (e: Exception) {
                if (isActive) {
                    Log.w(TAG, "Read loop ended: ${e.message}")
                }
            }
            // Connection lost
            if (stateMachine.getState() == ConnectionState.Connected) {
                stateMachine.transition(ConnectionState.Disconnected)
            }
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
        private const val DEFAULT_PORT = 9742
        private const val READ_TIMEOUT_MS = 30_000L
    }
}
