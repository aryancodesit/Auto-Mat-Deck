package com.automatdeck.spike

import android.os.Bundle
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
import java.util.concurrent.TimeUnit

class MainActivity : AppCompatActivity() {

    private lateinit var btnScan: Button
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

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        btnScan = findViewById(R.id.btnScan)
        deviceList = findViewById(R.id.deviceList)
        statusText = findViewById(R.id.statusText)
        responseText = findViewById(R.id.responseText)

        deviceList.layoutManager = LinearLayoutManager(this)

        btnScan.setOnClickListener {
            scanForDevices()
        }
    }

    private fun scanForDevices() {
        statusText.text = "Scanning..."
        btnScan.isEnabled = false

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
        responseText.text = "Connecting..."

        val wsUrl = "ws://${device.host}:${device.port}"
        val request = Request.Builder().url(wsUrl).build()

        currentWebSocket?.close(1000, "New connection")
        currentWebSocket = httpClient.newWebSocket(request, object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                statusText.post { statusText.text = "Connected to ${device.name}" }

                // Send ping
                val ping = JSONObject().apply {
                    put("type", "ping")
                    put("timestamp", System.currentTimeMillis())
                }
                webSocket.send(ping.toString())
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                responseText.post {
                    try {
                        val json = JSONObject(text)
                        val msgType = json.optString("type", "unknown")
                        responseText.text = when (msgType) {
                            "pong" -> {
                                val elapsed = System.currentTimeMillis() - (json
                                    .optJSONObject("echo")
                                    ?.optLong("timestamp", 0) ?: 0L)
                                "PONG received (${elapsed}ms round-trip)"
                            }
                            "error" -> "Error: ${json.optString("message")}"
                            else -> text
                        }
                    } catch (e: Exception) {
                        responseText.text = text
                    }
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

    override fun onDestroy() {
        currentWebSocket?.close(1000, "Activity destroyed")
        super.onDestroy()
    }
}
