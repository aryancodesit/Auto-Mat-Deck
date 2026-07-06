package com.automatdeck.spike

import android.content.Context
import android.util.Log
import java.util.concurrent.CopyOnWriteArrayList

class DiscoveryManager(private val context: Context) {

    private val providers = CopyOnWriteArrayList<DiscoveryProvider>()
    private var lastDiscoveryLatencyMs: Long = -1L

    init {
        register(MdnsDiscoveryProvider(context))
    }

    fun register(provider: DiscoveryProvider) {
        providers.add(provider)
        Log.d(TAG, "Registered discovery provider: ${provider::class.java.simpleName}")
    }

    suspend fun discover(timeoutMs: Long = 5000L): List<DiscoveredDevice> {
        val seen = mutableSetOf<String>()
        val allDevices = mutableListOf<DiscoveredDevice>()
        val scanStartTime = System.nanoTime()

        for (provider in providers) {
            val devices = provider.discover(timeoutMs)
            for (device in devices) {
                val key = "${device.host}:${device.port}"
                if (seen.add(key)) {
                    allDevices.add(device)
                }
            }
            if (allDevices.isNotEmpty()) {
                break
            }
        }

        lastDiscoveryLatencyMs = (System.nanoTime() - scanStartTime) / 1_000_000
        Log.i(TAG, "Discovery complete: ${allDevices.size} device(s) in ${lastDiscoveryLatencyMs}ms")
        return allDevices
    }

    fun getLastDiscoveryLatencyMs(): Long = lastDiscoveryLatencyMs

    companion object {
        private const val TAG = "DiscoveryManager"
    }
}
