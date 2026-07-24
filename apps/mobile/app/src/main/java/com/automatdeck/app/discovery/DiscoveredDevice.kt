package com.automatdeck.app.discovery

data class DiscoveredDevice(
    val name: String,
    val host: String,
    val port: Int,
    val deviceId: String,
    val protocolVersions: String,
    val discoveredAt: Long = System.currentTimeMillis()
) {
    val address: String get() = "$host:$port"

    companion object {
        fun unknown(host: String, port: Int) = DiscoveredDevice(
            name = "Unknown Desktop",
            host = host,
            port = port,
            deviceId = "",
            protocolVersions = ""
        )
    }
}
