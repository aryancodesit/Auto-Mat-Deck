package com.automatdeck.spike

import org.json.JSONObject

data class ControlInvokeRequest(
    val buttonId: String,
) {
    fun toJson(): JSONObject = JSONObject().apply {
        put("type", "control_invoke")
        put("schema_version", 1)
        put("button_id", buttonId)
    }
}
