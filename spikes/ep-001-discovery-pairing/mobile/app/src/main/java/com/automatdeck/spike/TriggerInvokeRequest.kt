package com.automatdeck.spike

import org.json.JSONObject

data class TriggerInvokeRequest(
    val triggerId: String,
    val workflowId: String
) {
    fun toJson(): JSONObject = JSONObject().apply {
        put("type", "trigger_invoke")
        put("schema_version", 1)
        put("trigger_id", triggerId)
        put("workflow_id", workflowId)
    }
}
