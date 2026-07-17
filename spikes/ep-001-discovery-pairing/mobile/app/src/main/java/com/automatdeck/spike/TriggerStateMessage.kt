package com.automatdeck.spike

import org.json.JSONArray
import org.json.JSONObject

data class TriggerStateMessage(
    val schemaVersion: Int,
    val triggers: List<TriggerMessage>
) {
    companion object {
        private const val EXPECTED_SCHEMA_VERSION = 1

        fun fromJson(json: JSONObject): TriggerStateMessage? {
            if (!json.has("schema_version")) return null
            val rawSchemaVersion = json.opt("schema_version")
            val schemaVersion = when (rawSchemaVersion) {
                is Int -> rawSchemaVersion
                is Long -> {
                    if (rawSchemaVersion !in Int.MIN_VALUE.toLong()..Int.MAX_VALUE.toLong()) {
                        return null
                    }
                    rawSchemaVersion.toInt()
                }
                else -> return null
            }
            if (schemaVersion != EXPECTED_SCHEMA_VERSION) return null

            if (!json.has("triggers")) return null
            val rawTriggers = json.opt("triggers")
            val triggers = when (rawTriggers) {
                is JSONArray -> {
                    val result = mutableListOf<TriggerMessage>()
                    for (i in 0 until rawTriggers.length()) {
                        val triggerObj = rawTriggers.optJSONObject(i) ?: return null
                        val trigger = TriggerMessage.fromJson(triggerObj) ?: return null
                        result.add(trigger)
                    }
                    result
                }
                else -> return null
            }

            return TriggerStateMessage(
                schemaVersion = schemaVersion,
                triggers = triggers
            )
        }
    }
}

data class TriggerMessage(
    val triggerId: String,
    val name: String,
    val type: String,
    val workflowId: String,
    val enabled: Boolean
) {
    companion object {
        fun fromJson(json: JSONObject): TriggerMessage? {
            if (!json.has("trigger_id")) return null
            val triggerId = json.optString("trigger_id", "")

            if (!json.has("name")) return null
            val name = json.optString("name", "")

            if (!json.has("type")) return null
            val type = json.optString("type", "")

            if (!json.has("workflow_id")) return null
            val workflowId = json.optString("workflow_id", "")

            if (!json.has("enabled")) return null
            val enabled = json.optBoolean("enabled", false)

            return TriggerMessage(
                triggerId = triggerId,
                name = name,
                type = type,
                workflowId = workflowId,
                enabled = enabled
            )
        }
    }
}
