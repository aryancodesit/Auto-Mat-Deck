package com.automatdeck.app.pairing

import android.content.Context
import android.content.SharedPreferences
import android.util.Log
import com.automatdeck.app.discovery.DiscoveredDevice

class TrustedDeviceStore(context: Context) {

    private val prefs: SharedPreferences =
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    fun save(device: DiscoveredDevice) {
        prefs.edit()
            .putString(KEY_HOST, device.host)
            .putInt(KEY_PORT, device.port)
            .putString(KEY_NAME, device.name)
            .putString(KEY_DEVICE_ID, device.deviceId)
            .putLong(KEY_PAIRED_AT, System.currentTimeMillis())
            .apply()
        Log.d(TAG, "Saved trusted device: ${device.name} (${device.address})")
    }

    fun get(): DiscoveredDevice? {
        val host = prefs.getString(KEY_HOST, null) ?: return null
        val port = prefs.getInt(KEY_PORT, 0)
        if (port == 0) return null
        return DiscoveredDevice(
            name = prefs.getString(KEY_NAME, "Unknown Desktop") ?: "Unknown Desktop",
            host = host,
            port = port,
            deviceId = prefs.getString(KEY_DEVICE_ID, "") ?: "",
            protocolVersions = ""
        )
    }

    fun isTrusted(deviceId: String): Boolean {
        val stored = prefs.getString(KEY_DEVICE_ID, null)
        return stored == deviceId
    }

    fun clear() {
        prefs.edit().clear().apply()
        Log.d(TAG, "Cleared trusted device store")
    }

    fun hasTrustedDevice(): Boolean {
        return prefs.getString(KEY_HOST, null) != null && prefs.getInt(KEY_PORT, 0) != 0
    }

    companion object {
        private const val TAG = "TrustedDeviceStore"
        private const val PREFS_NAME = "trusted_device"
        private const val KEY_HOST = "host"
        private const val KEY_PORT = "port"
        private const val KEY_NAME = "name"
        private const val KEY_DEVICE_ID = "device_id"
        private const val KEY_PAIRED_AT = "paired_at"
    }
}
