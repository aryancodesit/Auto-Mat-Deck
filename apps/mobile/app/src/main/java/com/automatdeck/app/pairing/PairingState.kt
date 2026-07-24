package com.automatdeck.app.pairing

enum class PairingState {
    Idle,
    WaitingForCode,
    SendingRequest,
    WaitingForApproval,
    Paired,
    Failed,
    TimedOut
}

data class PairingResult(
    val success: Boolean,
    val state: PairingState,
    val message: String = "",
    val deviceName: String = ""
)
