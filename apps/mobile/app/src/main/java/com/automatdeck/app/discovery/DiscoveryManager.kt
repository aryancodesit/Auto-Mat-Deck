package com.automatdeck.app.discovery

import android.content.Context
import android.util.Log
import kotlinx.coroutines.*
import java.util.concurrent.CopyOnWriteArrayList

class DiscoveryManager(private val context: Context) {

    private val providers = CopyOnWriteArrayList<DiscoveryProvider>()
    private val cache = DiscoveryCache(context)
    private var discoveryJob: Job? = null
    private var lastDiscoveryLatencyMs: Long = -1L
    private var onDeviceFound: ((DiscoveredDevice) -> Unit)? = null
    private var onDiscoveryComplete: ((List<DiscoveredDevice>) -> Unit)? = null

    init {
        register(MdnsDiscoveryProvider(context))
    }

    fun register(provider: DiscoveryProvider) {
        providers.add(provider)
        Log.d(TAG, "Registered provider: ${provider::class.java.simpleName}")
    }

    fun onDeviceFound(listener: (DiscoveredDevice) -> Unit) {
        onDeviceFound = listener
    }

    fun onDiscoveryComplete(listener: (List<DiscoveredDevice>) -> Unit) {
        onDiscoveryComplete = listener
    }

    suspend fun discover(timeoutMs: Long = 5000L): List<DiscoveredDevice> {
        val seen = mutableSetOf<String>()
        val allDevices = mutableListOf<DiscoveredDevice>()
        val scanStartTime = System.nanoTime()

        for (provider in providers) {
            val devices = provider.discover(timeoutMs)
            for (device in devices) {
                val key = device.address
                if (seen.add(key)) {
                    allDevices.add(device)
                    onDeviceFound?.invoke(device)
                }
            }
            if (allDevices.isNotEmpty()) {
                break
            }
        }

        lastDiscoveryLatencyMs = (System.nanoTime() - scanStartTime) / 1_000_000
        Log.i(TAG, "Discovery complete: ${allDevices.size} device(s) in ${lastDiscoveryLatencyMs}ms")

        // Cache the first device found for fast reconnection
        allDevices.firstOrNull()?.let { cache.saveLastKnown(it) }

        onDiscoveryComplete?.invoke(allDevices)
        return allDevices
    }

    fun startContinuousScan(
        scope: CoroutineScope,
        intervalMs: Long = 10_000L,
        timeoutMs: Long = 5000L
    ) {
        stopContinuousScan()
        discoveryJob = scope.launch(Dispatchers.IO) {
            while (isActive) {
                try {
                    discover(timeoutMs)
                } catch (e: Exception) {
                    Log.w(TAG, "Scan failed: ${e.message}")
                }
                delay(intervalMs)
            }
        }
        Log.d(TAG, "Continuous scan started (interval=${intervalMs}ms)")
    }

    fun stopContinuousScan() {
        discoveryJob?.cancel()
        discoveryJob = null
        providers.forEach { it.stop() }
        Log.d(TAG, "Continuous scan stopped")
    }

    fun getLastKnown(): DiscoveredDevice? = cache.getLastKnown()

    fun clearCache() = cache.clear()

    fun getLastDiscoveryLatencyMs(): Long = lastDiscoveryLatencyMs

    fun isScanning(): Boolean = discoveryJob?.isActive == true

    companion object {
        private const val TAG = "DiscoveryManager"
    }
}
