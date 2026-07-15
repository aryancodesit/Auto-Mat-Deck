package com.automatdeck.spike

import org.json.JSONObject

data class ActiveProfileStateMessage(
    val schemaVersion: Int,
    val activeProfileId: String?
) {
    companion object {
        private const val EXPECTED_SCHEMA_VERSION = 1

        fun fromJson(json: JSONObject): ActiveProfileStateMessage? {
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

            if (!json.has("active_profile_id")) return null
            val rawId = json.opt("active_profile_id")
            val activeProfileId = when (rawId) {
                JSONObject.NULL -> null
                is String -> rawId
                else -> return null
            }

            return ActiveProfileStateMessage(
                schemaVersion = schemaVersion,
                activeProfileId = activeProfileId
            )
        }
    }
}
