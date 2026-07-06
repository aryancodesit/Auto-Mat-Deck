package com.automatdeck.spike

import android.os.Build
import android.os.Bundle
import android.view.View
import android.widget.Button
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

    private lateinit var btnScan: Button
    private lateinit var btnPair: Button
    private lateinit var deviceList: RecyclerView
    private lateinit var statusText: TextView
    private lateinit var responseText: TextView

    private val scope = CoroutineScope(Dispatchers.Main)
    private val httpClient = OkHttpClient.Builder()
        .connectTimeout(5, TimeUnit.SECONDS)
        .readTimeout(10, TimeUnit.SECONDS)
        .build()

    private val discoveredDevices = mutableListOf<DiscoveredDevice>()
    private var currentWebSocket: WebSocket? = null

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
        deviceList = findViewById(R.id.deviceList)
        statusText = findViewById(R.id.statusText)
        responseText = findViewById(R.id.responseText)

        deviceList.layoutManager = LinearLayoutManager(this)

        btnScan.setOnClickListener { scanForDevices() }
        btnPair.setOnClickListener { sendPairRequest() }

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
        btnPair.visibility = View.GONE

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
        statusText.text = "Connecting to ${device.name}..."
        responseText.text = "Identifying..."
        btnPair.visibility = View.GONE

        // Save for auto-reconnect on success
        val wsUrl = "ws://${device.host}:${device.port}"
        val request = Request.Builder().url(wsUrl).build()

        currentWebSocket?.close(1000, "New connection")
        currentWebSocket = httpClient.newWebSocket(request, object : WebSocketListener() {
            private var paired = false

            override fun onOpen(webSocket: WebSocket, response: Response) {
                val identify = JSONObject().apply {
                    put("type", "identify")
                    put("device_id", deviceId)
                    put("device_name", "Android-${Build.MODEL}")
                }
                webSocket.send(identify.toString())
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                try {
                    val json = JSONObject(text)
                    when (json.optString("type")) {
                        "trusted" -> {
                            paired = true
                            saveTrustedDevice(device)
                            statusText.post {
                                statusText.text = "Connected to ${device.name} ✓"
                                responseText.text = "Paired ✓"
                            }
                            sendPing(webSocket)
                        }

                        "untrusted" -> {
                            statusText.post {
                                statusText.text = "Not paired with ${device.name}"
                                responseText.text = "Tap Pair to continue"
                                btnPair.visibility = View.VISIBLE
                            }
                        }

                        "pair_accepted" -> {
                            paired = true
                            saveTrustedDevice(device)
                            statusText.post {
                                statusText.text = "Connected to ${device.name} ✓"
                                responseText.text = "Paired ✓"
                                btnPair.visibility = View.GONE
                            }
                            sendPing(webSocket)
                        }

                        "pair_rejected" -> {
                            statusText.post {
                                statusText.text = "Pairing rejected by ${device.name}"
                                responseText.text = "Rejected: ${json.optString("reason")}"
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
                            statusText.post {
                                responseText.text = "Error: ${json.optString("message")}"
                            }
                        }

                        else -> {
                            statusText.post { responseText.text = text }
                        }
                    }
                } catch (_: Exception) {
                    statusText.post { responseText.text = text }
                }
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                statusText.post {
                    statusText.text = "Connection failed: ${t.message}"
                    responseText.text = "FAILED"
                }
            }

            override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
                statusText.post { statusText.text = "Connection closing: $reason" }
            }

            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                statusText.post { statusText.text = "Connection closed: $reason" }
            }
        })
    }

    private fun sendPairRequest() {
        currentWebSocket?.let { ws ->
            statusText.text = "Requesting pairing..."
            responseText.text = "Waiting for desktop approval..."
            val pairReq = JSONObject().apply {
                put("type", "pair_request")
                put("device_id", deviceId)
                put("device_name", "Android-${Build.MODEL}")
            }
            ws.send(pairReq.toString())
        }
    }

    private fun sendPing(webSocket: WebSocket) {
        val ping = JSONObject().apply {
            put("type", "ping")
            put("timestamp", System.currentTimeMillis())
        }
        webSocket.send(ping.toString())
    }

    private fun saveTrustedDevice(device: DiscoveredDevice) {
        val prefs = getSharedPreferences("auto_mat_deck", MODE_PRIVATE)
        prefs.edit()
            .putString("trusted_host", device.host)
            .putInt("trusted_port", device.port)
            .putString("trusted_name", device.name)
            .apply()
    }

    override fun onDestroy() {
        currentWebSocket?.close(1000, "Activity destroyed")
        super.onDestroy()
    }
}
