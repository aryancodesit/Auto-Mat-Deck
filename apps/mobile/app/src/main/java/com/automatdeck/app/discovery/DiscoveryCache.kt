package com.automatdeck.app.discovery

import android.content.Context
import android.content.SharedPreferences

class DiscoveryCache(context: Context) {

    private val prefs: SharedPreferences =
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    fun saveLastKnown(device: DiscoveredDevice) {
        prefs.edit()
            .putString(KEY_HOST, device.host)
            .putInt(KEY_PORT, device.port)
            .putString(KEY_NAME, device.name)
            .putString(KEY_DEVICE_ID, device.deviceId)
            .putLong(KEY_DISCOVERED_AT, device.discoveredAt)
            .apply()
    }

    fun getLastKnown(): DiscoveredDevice? {
        val host = prefs.getString(KEY_HOST, null) ?: return null
        val port = prefs.getInt(KEY_PORT, 0)
        if (port == 0) return null
        return DiscoveredDevice(
            name = prefs.getString(KEY_NAME, "Unknown Desktop") ?: "Unknown Desktop",
            host = host,
            port = port,
            deviceId = prefs.getString(KEY_DEVICE_ID, "") ?: "",
            protocolVersions = "",
            discoveredAt = prefs.getLong(KEY_DISCOVERED_AT, 0)
        )
       }

    fun clear() {
        prefs.edit().clear().apply()
    }

    companion object {
        private const val PREFS_NAME = "discovery_cache"
        private const val KEY_HOST = "host"
        private const val KEY_PORT = "port"
        private const val KEY_NAME = "name"
        private const val KEY_DEVICE_ID = "device_id"
        private const val KEY_DISCOVERED_AT = "discovered_at"
    }
}
