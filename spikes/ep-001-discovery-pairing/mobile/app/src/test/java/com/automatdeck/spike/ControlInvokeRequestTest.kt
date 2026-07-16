package com.automatdeck.spike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ControlInvokeRequestTest {

    // D1: minimum message has exactly type, schema_version, button_id
    @Test
    fun serializes_minimum_message() {
        val json = ControlInvokeRequest("btn_test").toJson()
        assertEquals("control_invoke", json.getString("type"))
        assertEquals(1, json.getInt("schema_version"))
        assertEquals("btn_test", json.getString("button_id"))
        assertEquals(3, json.length())
    }

    // D2: no forbidden fields present
    @Test
    fun does_not_include_profile_metadata() {
        val json = ControlInvokeRequest("btn_test").toJson()
        assertFalse(json.has("profile_id"))
        assertFalse(json.has("page_id"))
        assertFalse(json.has("action"))
        assertFalse(json.has("action_type"))
        assertFalse(json.has("payload"))
    }

    // D3: opaque button_id survives exactly
    @Test
    fun opaque_button_id_preserved() {
        val json = ControlInvokeRequest("my/complex-id_123").toJson()
        assertEquals("my/complex-id_123", json.getString("button_id"))
    }

    // D4: empty button_id serialized as-is
    @Test
    fun empty_button_id_serialized() {
        val json = ControlInvokeRequest("").toJson()
        assertEquals("", json.getString("button_id"))
        assertEquals(3, json.length())
    }

    // D5: serialized to string produces valid JSON
    @Test
    fun to_string_is_valid_json() {
        val json = ControlInvokeRequest("wifi").toJson()
        val parsed = JSONObject(json.toString())
        assertEquals("control_invoke", parsed.getString("type"))
        assertEquals("wifi", parsed.getString("button_id"))
    }
}
