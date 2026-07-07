package com.automatdeck.spike

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.os.Handler
import android.os.Looper
import android.util.Log
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlin.coroutines.resume

class MdnsDiscoveryProvider(private val context: Context) : DiscoveryProvider {

    private val nsdManager = context.getSystemService(Context.NSD_SERVICE) as NsdManager
    private val mainHandler = Handler(Looper.getMainLooper())

    override suspend fun discover(timeoutMs: Long): List<DiscoveredDevice> {
        val results = mutableListOf<DiscoveredDevice>()

        return suspendCancellableCoroutine { continuation ->
            var discoveryListener: NsdManager.DiscoveryListener? = null

            // Diagnostic: pre-discovery state
            val wifiManager = context.getSystemService(Context.WIFI_SERVICE) as? android.net.wifi.WifiManager
            val wifiInfo = wifiManager?.connectionInfo
            val ssid = wifiInfo?.ssid ?: "unknown"
            val ipInt = wifiInfo?.ipAddress ?: 0
            val ipString = "${(ipInt and 0xFF)}.${(ipInt shr 8 and 0xFF)}.${(ipInt shr 16 and 0xFF)}.${(ipInt shr 24 and 0xFF)}"
            Log.i(TAG, "Pre-discovery: type=$SERVICE_TYPE protocol=PROTOCOL_DNS_SD (${NsdManager.PROTOCOL_DNS_SD}) WiFi=$ssid IP=$ipString")

            val finishDiscovery: () -> Unit = {
                discoveryListener?.let { listener ->
                    try { nsdManager.stopServiceDiscovery(listener) } catch (_: Exception) {}
                }
                Log.i(TAG, "Discovery finished: ${results.size} device(s) found")
                if (!continuation.isCompleted) {
                    continuation.resume(results.toList())
                }
            }

            val timeoutRunnable = Runnable { finishDiscovery() }

            val listener = object : NsdManager.DiscoveryListener {
                override fun onDiscoveryStarted(serviceType: String) {
                    Log.i(TAG, "Discovery started: type=$serviceType")
                }

                override fun onServiceFound(serviceInfo: NsdServiceInfo) {
                    Log.i(TAG, "Service found: name=${serviceInfo.serviceName} type=${serviceInfo.serviceType}")
                    nsdManager.resolveService(serviceInfo, object : NsdManager.ResolveListener {
                        override fun onResolveFailed(info: NsdServiceInfo, errorCode: Int) {
                            Log.w(TAG, "Resolve failed: ${info.serviceName} error=$errorCode")
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
                            Log.i(TAG, "Resolved: ${device.host}:${device.port} name=${device.name} (t=${System.currentTimeMillis()})")
                            results.add(device)
                        }
                    })
                }

                override fun onServiceLost(serviceInfo: NsdServiceInfo) {
                    Log.i(TAG, "Service lost: ${serviceInfo.serviceName}")
                }

                override fun onDiscoveryStopped(serviceType: String) {
                    Log.i(TAG, "Discovery stopped: type=$serviceType")
                }

                override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
                    Log.e(TAG, "mDNS start discovery failed: error=$errorCode")
                    mainHandler.removeCallbacks(timeoutRunnable)
                    finishDiscovery()
                }

                override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) {
                    Log.e(TAG, "mDNS stop discovery failed: error=$errorCode")
                }
            }

            discoveryListener = listener
            nsdManager.discoverServices(SERVICE_TYPE, NsdManager.PROTOCOL_DNS_SD, listener)
            mainHandler.postDelayed(timeoutRunnable, timeoutMs)

            continuation.invokeOnCancellation {
                mainHandler.removeCallbacks(timeoutRunnable)
                try { nsdManager.stopServiceDiscovery(listener) } catch (_: Exception) {}
            }
        }
    }

    companion object {
        private const val TAG = "MdnsDiscoveryProvider"
        const val SERVICE_TYPE = "_amd._tcp."
    }
}
