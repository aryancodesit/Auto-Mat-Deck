package com.automatdeck.spike

import org.json.JSONArray
import org.json.JSONObject

enum class TriggerHistoryStatus {
    Success,
    Failed,
    Rejected;

    companion object {
        fun fromJson(value: Any?): TriggerHistoryStatus? {
            return when (value) {
                is String -> when (value) {
                    "Success" -> Success
                    else -> null
                }
                is JSONObject -> when {
                    value.has("Failed") -> Failed
                    value.has("Rejected") -> Rejected
                    else -> null
                }
                else -> null
            }
        }
    }
}

data class TriggerHistoryRecord(
    val triggerId: String,
    val workflowId: String,
    val status: TriggerHistoryStatus,
    val statusReason: String?,
    val timestamp: Long,
    val durationMs: Long,
) {
    companion object {
        fun fromJson(json: JSONObject): TriggerHistoryRecord? {
            val triggerId = json.optString("trigger_id", "")
            val workflowId = json.optString("workflow_id", "")

            val rawStatus = json.opt("status") ?: return null
            val status = TriggerHistoryStatus.fromJson(rawStatus) ?: return null

            val statusReason = when (rawStatus) {
                is JSONObject -> {
                    val inner = rawStatus.optJSONObject(
                        when (status) {
                            TriggerHistoryStatus.Failed -> "Failed"
                            TriggerHistoryStatus.Rejected -> "Rejected"
                            else -> return null
                        }
                    )
                    inner?.opt("reason")?.toString()
                }
                else -> null
            }

            val timestamp = json.optLong("timestamp", 0)
            val durationMs = json.optLong("duration_ms", 0)

            return TriggerHistoryRecord(
                triggerId = triggerId,
                workflowId = workflowId,
                status = status,
                statusReason = statusReason,
                timestamp = timestamp,
                durationMs = durationMs,
            )
        }
    }
}

data class TriggerHistoryMessage(
    val schemaVersion: Int,
    val records: List<TriggerHistoryRecord>,
) {
    companion object {
        private const val EXPECTED_SCHEMA_VERSION = 1

        fun fromJson(json: JSONObject): TriggerHistoryMessage? {
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

            if (!json.has("records")) return null
            val rawRecords = json.opt("records")
            val records = when (rawRecords) {
                is JSONArray -> {
                    val result = mutableListOf<TriggerHistoryRecord>()
                    for (i in 0 until rawRecords.length()) {
                        val recordObj = rawRecords.optJSONObject(i) ?: return null
                        val record = TriggerHistoryRecord.fromJson(recordObj) ?: return null
                        result.add(record)
                    }
                    result
                }
                else -> return null
            }

            return TriggerHistoryMessage(
                schemaVersion = schemaVersion,
                records = records,
            )
        }
    }
}
