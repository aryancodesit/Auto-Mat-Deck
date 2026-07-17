package com.automatdeck.spike

import org.json.JSONArray
import org.json.JSONObject

data class ControlSurfaceStateMessage(
    val schemaVersion: Int,
    val profileId: String?,
    val profileName: String?,
    val pages: List<PageMessage>?
) {
    companion object {
        private const val EXPECTED_SCHEMA_VERSION = 1

        fun fromJson(json: JSONObject): ControlSurfaceStateMessage? {
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

            if (!json.has("profile_id")) return null
            val rawProfileId = json.opt("profile_id")
            val profileId = when (rawProfileId) {
                JSONObject.NULL -> null
                is String -> rawProfileId
                else -> return null
            }

            if (!json.has("profile_name")) return null
            val rawProfileName = json.opt("profile_name")
            val profileName = when (rawProfileName) {
                JSONObject.NULL -> null
                is String -> rawProfileName
                else -> return null
            }

            if (!json.has("pages")) return null
            val rawPages = json.opt("pages")
            val pages = when (rawPages) {
                JSONObject.NULL -> null
                is JSONArray -> {
                    val result = mutableListOf<PageMessage>()
                    for (i in 0 until rawPages.length()) {
                        val pageObj = rawPages.optJSONObject(i) ?: return null
                        val page = PageMessage.fromJson(pageObj) ?: return null
                        result.add(page)
                    }
                    result
                }
                else -> return null
            }

            return ControlSurfaceStateMessage(
                schemaVersion = schemaVersion,
                profileId = profileId,
                profileName = profileName,
                pages = pages
            )
        }
    }
}

data class PageMessage(
    val pageId: String,
    val name: String,
    val buttons: List<ButtonMessage>
) {
    companion object {
        fun fromJson(json: JSONObject): PageMessage? {
            if (!json.has("page_id")) return null
            val pageId = json.optString("page_id", "")

            if (!json.has("name")) return null
            val name = json.optString("name", "")

            if (!json.has("buttons")) return null
            val rawButtons = json.opt("buttons")
            val buttons = when (rawButtons) {
                is JSONArray -> {
                    val result = mutableListOf<ButtonMessage>()
                    for (i in 0 until rawButtons.length()) {
                        val btnObj = rawButtons.optJSONObject(i) ?: return null
                        val button = ButtonMessage.fromJson(btnObj) ?: return null
                        result.add(button)
                    }
                    result
                }
                else -> return null
            }

            return PageMessage(pageId = pageId, name = name, buttons = buttons)
        }
    }
}

data class ButtonMessage(
    val buttonId: String,
    val label: String
) {
    companion object {
        fun fromJson(json: JSONObject): ButtonMessage? {
            if (!json.has("button_id")) return null
            val buttonId = json.optString("button_id", "")

            if (!json.has("label")) return null
            val label = json.optString("label", "")

            return ButtonMessage(buttonId = buttonId, label = label)
        }
    }
}
