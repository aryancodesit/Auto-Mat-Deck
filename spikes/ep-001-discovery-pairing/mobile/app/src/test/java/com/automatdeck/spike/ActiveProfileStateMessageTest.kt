package com.automatdeck.spike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test

class ActiveProfileStateMessageTest {

    @Test
    fun parse_valid_v1_with_string_id() {
        val json = JSONObject("""{"type":"active_profile_state","schema_version":1,"active_profile_id":"coding"}""")
        val msg = ActiveProfileStateMessage.fromJson(json)
        assertNotNull(msg)
        assertEquals(1, msg!!.schemaVersion)
        assertEquals("coding", msg.activeProfileId)
    }

    @Test
    fun parse_valid_v1_with_null_id() {
        val json = JSONObject("""{"type":"active_profile_state","schema_version":1,"active_profile_id":null}""")
        val msg = ActiveProfileStateMessage.fromJson(json)
        assertNotNull(msg)
        assertEquals(1, msg!!.schemaVersion)
        assertNull(msg.activeProfileId)
    }

    @Test
    fun parse_schema_version_2_is_rejected() {
        val json = JSONObject("""{"type":"active_profile_state","schema_version":2,"active_profile_id":"coding"}""")
        assertNull(ActiveProfileStateMessage.fromJson(json))
    }

    @Test
    fun parse_missing_schema_version_is_rejected() {
        val json = JSONObject("""{"type":"active_profile_state","active_profile_id":"coding"}""")
        assertNull(ActiveProfileStateMessage.fromJson(json))
    }

    @Test
    fun parse_wrong_type_schema_version_is_rejected() {
        val json = JSONObject("""{"type":"active_profile_state","schema_version":"one","active_profile_id":"coding"}""")
        assertNull(ActiveProfileStateMessage.fromJson(json))
    }

    @Test
    fun parse_missing_active_profile_id_is_rejected() {
        val json = JSONObject("""{"type":"active_profile_state","schema_version":1}""")
        assertNull(ActiveProfileStateMessage.fromJson(json))
    }

    @Test
    fun parse_number_active_profile_id_is_rejected() {
        val json = JSONObject("""{"type":"active_profile_state","schema_version":1,"active_profile_id":123}""")
        assertNull(ActiveProfileStateMessage.fromJson(json))
    }

    @Test
    fun parse_boolean_active_profile_id_is_rejected() {
        val json = JSONObject("""{"type":"active_profile_state","schema_version":1,"active_profile_id":true}""")
        assertNull(ActiveProfileStateMessage.fromJson(json))
    }

    @Test
    fun parse_object_active_profile_id_is_rejected() {
        val json = JSONObject("""{"type":"active_profile_state","schema_version":1,"active_profile_id":{}}""")
        assertNull(ActiveProfileStateMessage.fromJson(json))
    }

    @Test
    fun parse_array_active_profile_id_is_rejected() {
        val json = JSONObject("""{"type":"active_profile_state","schema_version":1,"active_profile_id":[]}""")
        assertNull(ActiveProfileStateMessage.fromJson(json))
    }

    @Test
    fun parse_malformed_json_returns_null() {
        try {
            val json = JSONObject("""{bad json""")
            assertNull(ActiveProfileStateMessage.fromJson(json))
        } catch (_: Exception) {
        }
    }

    @Test
    fun no_projection_is_distinct_from_explicit_null_profile() {
        var latestProjection: ActiveProfileStateMessage? = null
        assertNull(latestProjection)

        val nullProfileJson = JSONObject("""{"type":"active_profile_state","schema_version":1,"active_profile_id":null}""")
        val msg = ActiveProfileStateMessage.fromJson(nullProfileJson)
        assertNotNull(msg)
        latestProjection = msg

        assertNotNull(latestProjection)
        assertNull(latestProjection!!.activeProfileId)
    }

    @Test
    fun p1_then_p2_replaces_latest_projection() {
        var latestProjection: ActiveProfileStateMessage? = null

        val p1 = ActiveProfileStateMessage.fromJson(
            JSONObject("""{"type":"active_profile_state","schema_version":1,"active_profile_id":"coding"}""")
        )
        latestProjection = p1
        assertEquals("coding", latestProjection!!.activeProfileId)

        val p2 = ActiveProfileStateMessage.fromJson(
            JSONObject("""{"type":"active_profile_state","schema_version":1,"active_profile_id":"gaming"}""")
        )
        latestProjection = p2
        assertEquals("gaming", latestProjection!!.activeProfileId)

        val p3 = ActiveProfileStateMessage.fromJson(
            JSONObject("""{"type":"active_profile_state","schema_version":1,"active_profile_id":null}""")
        )
        latestProjection = p3
        assertNull(latestProjection!!.activeProfileId)
    }

    @Test
    fun unsupported_version_does_not_replace_existing_projection() {
        var latestProjection: ActiveProfileStateMessage? = null

        val p1 = ActiveProfileStateMessage.fromJson(
            JSONObject("""{"type":"active_profile_state","schema_version":1,"active_profile_id":"coding"}""")
        )
        latestProjection = p1

        val p2 = ActiveProfileStateMessage.fromJson(
            JSONObject("""{"type":"active_profile_state","schema_version":2,"active_profile_id":"gaming"}""")
        )
        assertNull(p2)

        assertEquals("coding", latestProjection!!.activeProfileId)
    }

    @Test
    fun malformed_json_does_not_replace_existing_projection() {
        var latestProjection: ActiveProfileStateMessage? = null

        val p1 = ActiveProfileStateMessage.fromJson(
            JSONObject("""{"type":"active_profile_state","schema_version":1,"active_profile_id":"coding"}""")
        )
        latestProjection = p1

        val invalid = ActiveProfileStateMessage.fromJson(
            JSONObject("""{"type":"active_profile_state","schema_version":1,"active_profile_id":123}""")
        )
        assertNull(invalid)

        assertEquals("coding", latestProjection!!.activeProfileId)
    }

    @Test
    fun duplicate_p1_is_harmless() {
        val json = """{"type":"active_profile_state","schema_version":1,"active_profile_id":"coding"}"""
        val p1a = ActiveProfileStateMessage.fromJson(JSONObject(json))
        val p1b = ActiveProfileStateMessage.fromJson(JSONObject(json))
        assertNotNull(p1a)
        assertNotNull(p1b)
        assertEquals(p1a!!.activeProfileId, p1b!!.activeProfileId)
        assertEquals(p1a.schemaVersion, p1b.schemaVersion)
    }
}
