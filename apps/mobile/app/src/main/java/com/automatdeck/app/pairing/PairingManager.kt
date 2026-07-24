package com.automatdeck.app.pairing

import android.os.Handler
import android.os.Looper
import android.util.Log
import org.json.JSONObject

class PairingManager(
    private val onStateChanged: (PairingState) -> Unit,
    private val onMessage: (String) -> Unit
) {
    private var currentState = PairingState.Idle
    private val timeoutHandler = Handler(Looper.getMainLooper())
    private var timeoutRunnable: Runnable? = null

    fun getState(): PairingState = currentState

    fun startPairing(sendFn: (String) -> Boolean) {
        if (currentState == PairingState.WaitingForApproval) {
            Log.w(TAG, "Already waiting for approval")
            return
        }
        updateState(PairingState.WaitingForCode)
        onMessage("Enter the 6-digit pairing code from Desktop")
    }

    fun sendPairRequest(
        code: String,
        deviceId: String,
        deviceName: String,
        sendFn: (String) -> Boolean
    ): Boolean {
        if (code.length != 6 || !code.all { it.isDigit() }) {
            onMessage("Enter a 6-digit pairing code")
            return false
        }

        updateState(PairingState.SendingRequest)
        onMessage("Requesting pairing...")

        val pairReq = JSONObject().apply {
            put("type", "pair_request")
            put("device_id", deviceId)
            put("device_name", deviceName)
            put("pairing_code", code)
        }

        val sent = sendFn(pairReq.toString())
        if (!sent) {
            updateState(PairingState.Failed)
            onMessage("Failed to send pair request")
            return false
        }

        updateState(PairingState.WaitingForApproval)
        startTimeout()
        return true
    }

    fun handlePairAccepted(deviceName: String) {
        cancelTimeout()
        updateState(Paired)
        onMessage("Paired with $deviceName ✓")
    }

    fun handlePairRejected(reason: String) {
        cancelTimeout()
        val message = when (reason) {
            "code_mismatch" -> "Invalid pairing code. Check the code and try again."
            "expired" -> "Pairing code expired. Generate a new code on Desktop."
            "cancelled" -> "Pairing was cancelled on Desktop."
            "already_used" -> "This pairing code has already been used."
            "no_session" -> "No active pairing session. Generate a code on Desktop."
            "user_declined" -> "Pairing was declined on Desktop."
            "timeout" -> "Pairing approval timed out."
            else -> "Pairing rejected ($reason). Try again."
        }
        updateState(PairingState.Failed)
        onMessage(message)
    }

    fun handleTrusted(deviceName: String) {
        cancelTimeout()
        updateState(PairingState.Paired)
        onMessage("Already paired with $deviceName ✓")
    }

    fun reset() {
        cancelTimeout()
        updateState(PairingState.Idle)
    }

    private fun startTimeout() {
        cancelTimeout()
        timeoutRunnable = Runnable {
            if (currentState == PairingState.WaitingForApproval) {
                updateState(PairingState.TimedOut)
                onMessage("Pairing timed out. Check the code and try again.")
            }
        }
        timeoutHandler.postDelayed(timeoutRunnable!!, PAIR_TIMEOUT_MS)
    }

    private fun cancelTimeout() {
        timeoutRunnable?.let { timeoutHandler.removeCallbacks(it) }
        timeoutRunnable = null
    }

    private fun updateState(newState: PairingState) {
        if (currentState != newState) {
            Log.d(TAG, "State: $currentState -> $newState")
            currentState = newState
            onStateChanged(newState)
        }
    }

    companion object {
        private const val TAG = "PairingManager"
        private const val PAIR_TIMEOUT_MS = 130_000L
    }
}
