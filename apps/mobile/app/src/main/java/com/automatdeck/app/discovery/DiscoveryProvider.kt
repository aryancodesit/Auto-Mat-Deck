package com.automatdeck.app.discovery

interface DiscoveryProvider {
    suspend fun discover(timeoutMs: Long = 5000L): List<DiscoveredDevice>
    fun stop() {}
}
