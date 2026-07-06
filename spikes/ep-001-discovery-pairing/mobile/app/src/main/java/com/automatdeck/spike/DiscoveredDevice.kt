package com.automatdeck.spike

data class DiscoveredDevice(
    val name: String,
    val host: String,
    val port: Int,
    val deviceId: String,
    val protocolVersions: String
)
