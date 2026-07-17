package com.automatdeck.spike

import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.text.Editable
import android.text.TextWatcher
import android.util.Log
import android.view.View
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import org.json.JSONObject
import java.util.UUID
import java.util.concurrent.TimeUnit

class MainActivity : AppCompatActivity() {
    companion object {
        private const val TAG = "AMD"
        private const val PAIR_TIMEOUT_MS = 130_000L
    }

    private lateinit var btnScan: Button
    private lateinit var btnPair: Button
    private lateinit var btnScanQR: Button
    private lateinit var txtPairingCode: EditText
    private lateinit var pairingCodeSection: View
    private lateinit var deviceList: RecyclerView
    private lateinit var statusText: TextView
    private lateinit var responseText: TextView
    private lateinit var cssContainer: LinearLayout

    private val scope = CoroutineScope(Dispatchers.Main)
    private val httpClient = OkHttpClient.Builder()
        .connectTimeout(5, TimeUnit.SECONDS)
        .readTimeout(10, TimeUnit.SECONDS)
        .build()

    private val discoveredDevices = mutableListOf<DiscoveredDevice>()
    private var currentWebSocket: WebSocket? = null
    private var currentDevice: DiscoveredDevice? = null

    private val dispatcher = SpikeMessageDispatcher()

    private val deviceId: String by lazy {
        val prefs = getSharedPreferences("auto_mat_deck", MODE_PRIVATE)
        var id = prefs.getString("device_id", null)
        if (id == null) {
            id = UUID.randomUUID().toString()
            prefs.edit().putString("device_id", id).apply()
        }
        id
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        btnScan = findViewById(R.id.btnScan)
        btnPair = findViewById(R.id.btnPair)
        btnScanQR = findViewById(R.id.btnScanQR)
        txtPairingCode = findViewById(R.id.txtPairingCode)
        pairingCodeSection = findViewById(R.id.pairingCodeSection)
        deviceList = findViewById(R.id.deviceList)
        statusText = findViewById(R.id.statusText)
        responseText = findViewById(R.id.responseText)
        cssContainer = findViewById(R.id.cssContainer)

        deviceList.layoutManager = LinearLayoutManager(this)

        btnScan.setOnClickListener { scanForDevices() }
        btnPair.setOnClickListener { sendPairRequest() }
        btnScanQR.setOnClickListener { scanQR() }

        // Enable Pair button only when 6 digits are typed
        txtPairingCode.addTextChangedListener(object : TextWatcher {
            override fun afterTextChanged(s: Editable?) {
                val text = s.toString().trim()
                btnPair.isEnabled = text.length == 6 && text.all { it.isDigit() }
            }
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) {}
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {}
        })

        autoReconnect()
    }

    private fun autoReconnect() {
        val prefs = getSharedPreferences("auto_mat_deck", MODE_PRIVATE)
        val host = prefs.getString("trusted_host", null)
        val port = prefs.getInt("trusted_port", 0)
        val name = prefs.getString("trusted_name", null)
        if (host != null && port > 0 && name != null) {
            statusText.text = "Auto-connecting to $name..."
            connectToDevice(DiscoveredDevice(name, host, port, "", ""))
        }
    }

    private fun scanForDevices() {
        statusText.text = "Scanning..."
        btnScan.isEnabled = false
        pairingCodeSection.visibility = View.GONE

        scope.launch {
            val discoveryManager = DiscoveryManager(this@MainActivity)
            val devices = discoveryManager.discover()

            discoveredDevices.clear()
            discoveredDevices.addAll(devices)

            deviceList.adapter = DeviceAdapter(devices) { device ->
                connectToDevice(device)
            }

            val latencyMs = discoveryManager.getLastDiscoveryLatencyMs()
            val latencyText = if (latencyMs >= 0) " (${latencyMs}ms)" else ""

            statusText.text = if (devices.isEmpty()) {
                "No desktops found. Make sure the desktop agent is running."
            } else {
                "Found ${devices.size} desktop(s). Tap one to connect.$latencyText"
            }

            btnScan.isEnabled = true
        }
    }

    private fun connectToDevice(device: DiscoveredDevice) {
        currentDevice = device
        statusText.text = "Connecting to ${device.name}..."
        responseText.text = "Identifying..."
        pairingCodeSection.visibility = View.GONE
        btnPair.isEnabled = false
        txtPairingCode.text.clear()

        val wsUrl = "ws://${device.host}:${device.port}"
        Log.i(TAG, "Connecting to $wsUrl (device=${device.name})")
        val request = Request.Builder().url(wsUrl).build()

        currentWebSocket?.close(1000, "New connection")
        currentWebSocket = httpClient.newWebSocket(request, object : WebSocketListener() {
            private var paired = false

            override fun onOpen(webSocket: WebSocket, response: Response) {
                Log.i(TAG, "WebSocket connected to ${device.name}, sending identify")
                val identify = JSONObject().apply {
                    put("type", "identify")
                    put("device_id", deviceId)
                    put("device_name", "Android-${Build.MODEL}")
                }
                webSocket.send(identify.toString())
                Log.i(TAG, "Identify sent to ${device.name}")
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                try {
                    val json = JSONObject(text)
                    val msgType = json.optString("type")
                    Log.i(TAG, "Received message type=$msgType from ${device.name}")
                    when (msgType) {
                        "trusted" -> {
                            Log.i(TAG, "Trusted by ${device.name} — already paired")
                            paired = true
                            saveTrustedDevice(device)
                            statusText.post {
                                statusText.text = "Connected to ${device.name} ✓"
                                responseText.text = "Paired ✓"
                                pairingCodeSection.visibility = View.GONE
                            }
                            sendPing(webSocket)
                        }

                        "untrusted" -> {
                            Log.i(TAG, "Untrusted by ${device.name} — show pairing code input")
                            statusText.post {
                                statusText.text = "Not paired with ${device.name}"
                                responseText.text = "Enter the 6-digit pairing code from Desktop"
                                pairingCodeSection.visibility = View.VISIBLE
                                txtPairingCode.requestFocus()
                                // Enable Pair button if code is already typed
                                val code = txtPairingCode.text.toString().trim()
                                btnPair.isEnabled = code.length == 6 && code.all { it.isDigit() }
                            }
                        }

                        "pair_accepted" -> {
                            Log.i(TAG, "Pair ACCEPTED by ${device.name}")
                            paired = true
                            saveTrustedDevice(device)
                            statusText.post {
                                statusText.text = "Connected to ${device.name} ✓"
                                responseText.text = "Paired ✓"
                                pairingCodeSection.visibility = View.GONE
                            }
                            sendPing(webSocket)
                        }

                        "pair_rejected" -> {
                            val rawReason = json.optString("reason", "")
                            val uiMessage = when (rawReason) {
                                "code_mismatch" -> "Invalid pairing code. Check the code and try again."
                                "expired" -> "Pairing code expired. Generate a new code on Desktop."
                                "cancelled" -> "Pairing was cancelled on Desktop."
                                "already_used" -> "This pairing code has already been used."
                                "no_session" -> "No active pairing session. Generate a code on Desktop."
                                "user_declined" -> "Pairing was declined on Desktop."
                                "timeout" -> "Pairing approval timed out."
                                else -> {
                                    Log.w(TAG, "Unknown pair_rejected reason code: $rawReason")
                                    "Pairing rejected ($rawReason). Try again."
                                }
                            }
                            Log.i(TAG, "Pair REJECTED by ${device.name}: reason=$rawReason -> $uiMessage")
                            statusText.post {
                                statusText.text = "Pairing rejected by ${device.name}"
                                responseText.text = uiMessage
                                // Re-enable pairing for retry
                                pairingCodeSection.visibility = View.VISIBLE
                                btnPair.isEnabled = false
                            }
                        }

                        "pong" -> {
                            val elapsed = System.currentTimeMillis() - (json
                                .optJSONObject("echo")
                                ?.optLong("timestamp", 0) ?: 0L)
                            statusText.post {
                                responseText.text = "PONG received (${elapsed}ms round-trip)"
                            }
                        }

                        "error" -> {
                            Log.w(TAG, "Error from ${device.name}: ${json.optString("message")}")
                            statusText.post {
                                responseText.text = "Error: ${json.optString("message")}"
                            }
                        }

                        "control_invoke_result" -> {
                            dispatcher.handle(text)
                            val result = dispatcher.lastInvokeResult
                            Log.i(TAG, "control_invoke_result: button_id=${result?.buttonId}, accepted=${result?.accepted}, executed=${result?.executed}")
                            statusText.post {
                                if (result != null) {
                                    responseText.text = when {
                                        !result.accepted -> "Invoke rejected: ${result.buttonId} — ${result.reason ?: "unknown"}"
                                        result.executed == true -> "Invoke accepted: ${result.buttonId} — executed"
                                        else -> "Invoke accepted: ${result.buttonId} — ${result.executionError ?: "execution_failed"}"
                                    }
                                }
                            }
                        }

                        "active_profile_state",
                        "control_surface_state" -> {
                            dispatcher.handle(text)
                            Log.i(TAG, "Dispatched: type=$msgType, state=${dispatcher.uiState}")
                            statusText.post { renderUiState() }
                        }

                        else -> {
                            Log.w(TAG, "Unknown message type=$msgType from ${device.name}: $text")
                            statusText.post { responseText.text = text }
                        }
                    }
                } catch (e: Exception) {
                    Log.e(TAG, "Failed to parse message from ${device.name}", e)
                    statusText.post { responseText.text = text }
                }
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                val detail = "${t::class.simpleName}: ${t.message}"
                Log.e(TAG, "Connection FAILED for ${device.name}: $detail", t)
                statusText.post {
                    statusText.text = "Connection failed: $detail"
                    responseText.text = "FAILED"
                    btnPair.isEnabled = false
                    pairingCodeSection.visibility = View.GONE
                }
            }

            override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
                Log.i(TAG, "Connection closing for ${device.name}: ($code) $reason")
                statusText.post { statusText.text = "Connection closing: $reason" }
            }

            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                Log.i(TAG, "Connection closed for ${device.name}: ($code) $reason")
                statusText.post {
                    statusText.text = "Connection closed: $reason"
                    btnPair.isEnabled = false
                    pairingCodeSection.visibility = View.GONE
                }
            }
        })
    }

    private fun sendPairRequest() {
        val ws = currentWebSocket
        if (ws == null) {
            Log.w(TAG, "sendPairRequest: currentWebSocket is null")
            statusText.text = "Not connected. Scan and connect first."
            return
        }
        val code = txtPairingCode.text.toString().trim()
        if (code.length != 6 || !code.all { it.isDigit() }) {
            responseText.text = "Enter a 6-digit pairing code from Desktop"
            return
        }

        Log.i(TAG, "Sending pair_request (device_id=$deviceId, pairing_code=$code)")
        statusText.text = "Requesting pairing..."
        responseText.text = "Waiting for desktop approval..."
        btnPair.isEnabled = false

        val pairReq = JSONObject().apply {
            put("type", "pair_request")
            put("device_id", deviceId)
            put("device_name", "Android-${Build.MODEL}")
            put("pairing_code", code)
        }
        val sent = ws.send(pairReq.toString())
        Log.i(TAG, "pair_request sent, result=$sent")

        // Post a timeout handler so the UI doesn't wait forever
        Handler(Looper.getMainLooper()).postDelayed({
            statusText.post {
                if (statusText.text == "Requesting pairing..." ||
                    statusText.text == "Waiting for desktop approval..."
                ) {
                    statusText.text = "Pairing timed out. Check the code and try again."
                    responseText.text = "TIMEOUT"
                    btnPair.isEnabled = true
                }
            }
        }, PAIR_TIMEOUT_MS)
    }

    private fun scanQR() {
        Log.i(TAG, "QR scan requested — not yet implemented")
        val code = txtPairingCode.text.toString().trim()
        if (code.length == 6 && code.all { it.isDigit() }) {
            sendPairRequest()
        } else {
            responseText.text = "QR scanning not available yet. Enter the code manually."
        }
        // ponytail: QR via ML Kit deferred until production mobile app (v0.3).
        // Spike stays OTP-only for now.
    }

    private fun sendPing(webSocket: WebSocket) {
        val ping = JSONObject().apply {
            put("type", "ping")
            put("timestamp", System.currentTimeMillis())
        }
        Log.i(TAG, "Sending ping")
        webSocket.send(ping.toString())
    }

    private fun sendControlInvoke(buttonId: String) {
        val ws = currentWebSocket
        if (ws == null) {
            Log.w(TAG, "sendControlInvoke: currentWebSocket is null")
            return
        }
        val invoke = ControlInvokeRequest(buttonId).toJson()
        Log.i(TAG, "Sending control_invoke: button_id=$buttonId")
        ws.send(invoke.toString())
    }

    private fun saveTrustedDevice(device: DiscoveredDevice) {
        val prefs = getSharedPreferences("auto_mat_deck", MODE_PRIVATE)
        prefs.edit()
            .putString("trusted_host", device.host)
            .putInt("trusted_port", device.port)
            .putString("trusted_name", device.name)
            .apply()
    }

    private fun renderUiState() {
        cssContainer.removeAllViews()
        val items = ControlSurfacePresentationMapper.map(dispatcher.uiState)
        for (item in items) {
            when (item) {
                is ControlSurfacePresentationItem.NoContent -> {
                    val tv = TextView(this).apply { text = item.label }
                    cssContainer.addView(tv)
                }
                is ControlSurfacePresentationItem.ProfileHeader -> {
                    val tv = TextView(this).apply {
                        text = item.profileName
                        textSize = 16f
                        setTextColor(0xFFFFA500.toInt())
                    }
                    cssContainer.addView(tv)
                }
                is ControlSurfacePresentationItem.PageHeader -> {
                    val tv = TextView(this).apply {
                        text = item.pageName
                        textSize = 14f
                        setTextColor(0xFFAAAAAA.toInt())
                    }
                    cssContainer.addView(tv)
                }
                is ControlSurfacePresentationItem.ButtonTile -> {
                    val b = Button(this).apply {
                        text = item.label
                        tag = item.buttonId
                        setOnClickListener {
                            Log.i(TAG, "Button pressed: ${item.buttonId} (${item.label})")
                            sendControlInvoke(item.buttonId)
                        }
                    }
                    cssContainer.addView(b)
                }
            }
        }
    }

    override fun onDestroy() {
        currentWebSocket?.close(1000, "Activity destroyed")
        super.onDestroy()
    }
}
