package com.automatdeck.spike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class TriggerInvokeRequestTest {

    // D1: minimum message has exactly type, schema_version, trigger_id, workflow_id
    @Test
    fun serializes_minimum_message() {
        val json = TriggerInvokeRequest("t1", "wf1").toJson()
        assertEquals("trigger_invoke", json.getString("type"))
        assertEquals(1, json.getInt("schema_version"))
        assertEquals("t1", json.getString("trigger_id"))
        assertEquals("wf1", json.getString("workflow_id"))
        assertEquals(4, json.length())
    }

    // D2: no forbidden fields present
    @Test
    fun does_not_include_extra_fields() {
        val json = TriggerInvokeRequest("t1", "wf1").toJson()
        assertFalse(json.has("button_id"))
        assertFalse(json.has("action"))
        assertFalse(json.has("action_type"))
        assertFalse(json.has("payload"))
    }

    // D3: opaque IDs preserved exactly
    @Test
    fun opaque_ids_preserved() {
        val json = TriggerInvokeRequest("my/complex-id_123", "wf/456").toJson()
        assertEquals("my/complex-id_123", json.getString("trigger_id"))
        assertEquals("wf/456", json.getString("workflow_id"))
    }

    // D4: empty IDs serialized as-is
    @Test
    fun empty_ids_serialized() {
        val json = TriggerInvokeRequest("", "").toJson()
        assertEquals("", json.getString("trigger_id"))
        assertEquals("", json.getString("workflow_id"))
        assertEquals(4, json.length())
    }

    // D5: serialized to string produces valid JSON
    @Test
    fun to_string_is_valid_json() {
        val json = TriggerInvokeRequest("t1", "wf1").toJson()
        val parsed = JSONObject(json.toString())
        assertEquals("trigger_invoke", parsed.getString("type"))
        assertEquals("t1", parsed.getString("trigger_id"))
        assertEquals("wf1", parsed.getString("workflow_id"))
    }
}
