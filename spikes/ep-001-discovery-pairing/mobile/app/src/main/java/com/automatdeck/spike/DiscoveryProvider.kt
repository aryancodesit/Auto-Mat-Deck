package com.automatdeck.spike

interface DiscoveryProvider {
    suspend fun discover(timeoutMs: Long = 5000L): List<DiscoveredDevice>
}
