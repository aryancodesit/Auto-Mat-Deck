package com.automatdeck.spike

import android.os.Build
import android.os.Bundle
import android.util.Log
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
    companion object { private const val TAG = "AMD" }

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
        btnPair.isEnabled = false

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
                            }
                            sendPing(webSocket)
                        }

                        "untrusted" -> {
                            Log.i(TAG, "Untrusted by ${device.name} — awaiting pair_request")
                            statusText.post {
                                statusText.text = "Not paired with ${device.name}"
                                responseText.text = "Tap Pair to continue"
                                btnPair.visibility = View.VISIBLE
                                btnPair.isEnabled = true
                            }
                        }

                        "pair_accepted" -> {
                            Log.i(TAG, "Pair ACCEPTED by ${device.name}")
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
                            Log.w(TAG, "Pair REJECTED by ${device.name}: ${json.optString("reason")}")
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
                            Log.w(TAG, "Error from ${device.name}: ${json.optString("message")}")
                            statusText.post {
                                responseText.text = "Error: ${json.optString("message")}"
                            }
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
                    btnPair.visibility = View.GONE
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
                    btnPair.visibility = View.GONE
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
        Log.i(TAG, "Sending pair_request (device_id=$deviceId)")
        statusText.text = "Requesting pairing..."
        responseText.text = "Waiting for desktop approval..."
        val pairReq = JSONObject().apply {
            put("type", "pair_request")
            put("device_id", deviceId)
            put("device_name", "Android-${Build.MODEL}")
        }
        val sent = ws.send(pairReq.toString())
        Log.i(TAG, "pair_request sent, result=$sent")
    }

    private fun sendPing(webSocket: WebSocket) {
        val ping = JSONObject().apply {
            put("type", "ping")
            put("timestamp", System.currentTimeMillis())
        }
        Log.i(TAG, "Sending ping")
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
