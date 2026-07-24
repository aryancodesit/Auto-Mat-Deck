package com.automatdeck.app.discovery

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.os.Handler
import android.os.Looper
import android.util.Log
import kotlinx.coroutines.suspendCancellableCoroutine
import java.util.concurrent.CopyOnWriteArrayList
import kotlin.coroutines.resume

class MdnsDiscoveryProvider(context: Context) : DiscoveryProvider {

    private val nsdManager = context.getSystemService(Context.NSD_SERVICE) as NsdManager
    private val mainHandler = Handler(Looper.getMainLooper())
    private var currentListener: NsdManager.DiscoveryListener? = null

    override suspend fun discover(timeoutMs: Long): List<DiscoveredDevice> {
        // ponytail: CopyOnWriteArrayList — onServiceResolved callbacks fire concurrently
        val results = CopyOnWriteArrayList<DiscoveredDevice>()

        return suspendCancellableCoroutine { continuation ->
            var discoveryListener: NsdManager.DiscoveryListener? = null

            val finishDiscovery: () -> Unit = {
                discoveryListener?.let { listener ->
                    try {
                        nsdManager.stopServiceDiscovery(listener)
                    } catch (_: Exception) {
                    }
                }
                currentListener = null
                if (!continuation.isCompleted) {
                    continuation.resume(results.toList())
                }
            }

            val timeoutRunnable = Runnable { finishDiscovery() }

            val listener = object : NsdManager.DiscoveryListener {
                override fun onDiscoveryStarted(serviceType: String) {
                    Log.d(TAG, "Discovery started: $serviceType")
                }

                val resolveQueue = java.util.concurrent.ConcurrentLinkedQueue<NsdServiceInfo>()
                val isResolving = java.util.concurrent.atomic.AtomicBoolean(false)

                fun processNextResolve() {
                    if (!isResolving.compareAndSet(false, true)) return
                    val next = resolveQueue.poll()
                    if (next == null) {
                        isResolving.set(false)
                        return
                    }
                    try {
                        nsdManager.resolveService(next, object : NsdManager.ResolveListener {
                            override fun onResolveFailed(info: NsdServiceInfo, errorCode: Int) {
                                Log.w(TAG, "Resolve failed: ${info.serviceName} error=$errorCode")
                                isResolving.set(false)
                                processNextResolve()
                            }

                            override fun onServiceResolved(info: NsdServiceInfo) {
                                val device = DiscoveredDevice(
                                    name = info.serviceName,
                                    host = info.host.hostAddress ?: "unknown",
                                    port = info.port,
                                    deviceId = info.attributes["deviceId"]
                                        ?.let { String(it, Charsets.UTF_8) } ?: "unknown",
                                    protocolVersions = info.attributes["protocolVersions"]
                                        ?.let { String(it, Charsets.UTF_8) } ?: "unknown"
                                )
                                Log.d(TAG, "Resolved: ${device.address} name=${device.name}")
                                results.add(device)
                                isResolving.set(false)
                                processNextResolve()
                            }
                        })
                    } catch (e: Exception) {
                        Log.e(TAG, "Exception during resolveService", e)
                        isResolving.set(false)
                        processNextResolve()
                    }
                }

                override fun onServiceFound(serviceInfo: NsdServiceInfo) {
                    Log.d(TAG, "Service found: ${serviceInfo.serviceName}")
                    resolveQueue.offer(serviceInfo)
                    processNextResolve()
                }

                override fun onServiceLost(serviceInfo: NsdServiceInfo) {
                    Log.d(TAG, "Service lost: ${serviceInfo.serviceName}")
                }

                override fun onDiscoveryStopped(serviceType: String) {
                    Log.d(TAG, "Discovery stopped: $serviceType")
                }

                override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
                    Log.e(TAG, "Start discovery failed: error=$errorCode")
                    mainHandler.removeCallbacks(timeoutRunnable)
                    finishDiscovery()
                }

                override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) {
                    Log.e(TAG, "Stop discovery failed: error=$errorCode")
                }
            }

            discoveryListener = listener
            currentListener = listener
            nsdManager.discoverServices(SERVICE_TYPE, NsdManager.PROTOCOL_DNS_SD, listener)
            mainHandler.postDelayed(timeoutRunnable, timeoutMs)

            continuation.invokeOnCancellation {
                mainHandler.removeCallbacks(timeoutRunnable)
                try {
                    nsdManager.stopServiceDiscovery(listener)
                } catch (_: Exception) {
                }
                currentListener = null
            }
        }
    }

    override fun stop() {
        currentListener?.let { listener ->
            try {
                nsdManager.stopServiceDiscovery(listener)
            } catch (_: Exception) {
            }
            currentListener = null
        }
    }

    companion object {
        private const val TAG = "MdnsDiscovery"
        const val SERVICE_TYPE = "_amd._tcp."
    }
}
