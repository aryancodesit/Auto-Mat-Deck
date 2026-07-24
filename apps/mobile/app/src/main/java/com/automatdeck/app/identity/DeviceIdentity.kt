package com.automatdeck.app.identity

import android.content.Context
import android.content.SharedPreferences
import android.util.Log
import java.util.UUID

/**
 * Stable device identity for the Android client.
 *
 * Generated once (UUID v4) on first launch, persisted permanently,
 * and never regenerated unless the user explicitly resets pairing.
 *
 * This identity is used for:
 * - Pairing (identify message)
 * - Trust verification (TrustedDeviceStore)
 * - Session restoration (auto-reconnect)
 * - Future: session IDs, heartbeat
 */
class DeviceIdentity(context: Context) {

    private val prefs: SharedPreferences =
        context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)

    var deviceId: String
        private set
    var deviceName: String
        private set

    init {
        val stored = prefs.getString(KEY_DEVICE_ID, null)
        if (stored != null) {
            deviceId = stored
            Log.d(TAG, "Loaded existing identity: $deviceId")
        } else {
            deviceId = generateUUID()
            prefs.edit().putString(KEY_DEVICE_ID, deviceId).commit()
            Log.i(TAG, "Generated new identity: $deviceId")
        }
        deviceName = prefs.getString(KEY_DEVICE_NAME, defaultName()) ?: defaultName()
    }

    fun setName(name: String) {
        deviceName = name
        prefs.edit().putString(KEY_DEVICE_NAME, name).apply()
    }

    /**
     * Reset identity. Only call when user explicitly resets pairing.
     * This generates a new UUID and invalidates all existing trust.
     * Both in-memory and persistent identity are updated atomically.
     */
    fun reset() {
        val newId = generateUUID()
        val oldId = deviceId
        deviceId = newId
        prefs.edit().putString(KEY_DEVICE_ID, newId).commit()
        Log.w(TAG, "Identity reset: $oldId -> $newId")
    }

    fun isPaired(): Boolean {
        return prefs.getBoolean(KEY_PAIRED, false)
    }

    fun setPaired(paired: Boolean) {
        prefs.edit().putBoolean(KEY_PAIRED, paired).apply()
    }

    private fun defaultName(): String {
        val model = android.os.Build.MODEL ?: "Android"
        return "Android-$model"
    }

    private fun generateUUID(): String {
        return UUID.randomUUID().toString()
    }

    companion object {
        private const val TAG = "DeviceIdentity"
        private const val PREFS_NAME = "device_identity"
        private const val KEY_DEVICE_ID = "device_id"
        private const val KEY_DEVICE_NAME = "device_name"
        private const val KEY_PAIRED = "paired"
    }
}
