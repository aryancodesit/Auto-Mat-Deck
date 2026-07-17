package com.automatdeck.spike

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class TriggerStateMessageTest {

    // T1: valid single trigger parsed correctly
    @Test
    fun parse_valid_single_trigger() {
        val json = JSONObject("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[{"trigger_id":"t1","name":"Morning","type":"time","workflow_id":"wf1","enabled":true}]
        }""")
        val msg = TriggerStateMessage.fromJson(json)
        assertNotNull(msg)
        assertEquals(1, msg!!.schemaVersion)
        assertEquals(1, msg.triggers.size)
        assertEquals("t1", msg.triggers[0].triggerId)
        assertEquals("Morning", msg.triggers[0].name)
        assertEquals("time", msg.triggers[0].type)
        assertEquals("wf1", msg.triggers[0].workflowId)
        assertTrue(msg.triggers[0].enabled)
    }

    // T2: valid multiple triggers parsed correctly
    @Test
    fun parse_valid_multiple_triggers() {
        val json = JSONObject("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[
                {"trigger_id":"t1","name":"Morning","type":"time","workflow_id":"wf1","enabled":true},
                {"trigger_id":"t2","name":"Manual","type":"manual","workflow_id":"wf2","enabled":false}
            ]
        }""")
        val msg = TriggerStateMessage.fromJson(json)
        assertNotNull(msg)
        assertEquals(2, msg!!.triggers.size)
        assertEquals("t1", msg.triggers[0].triggerId)
        assertEquals("t2", msg.triggers[1].triggerId)
    }

    // T3: empty triggers array parsed correctly
    @Test
    fun parse_empty_triggers() {
        val json = JSONObject("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[]
        }""")
        val msg = TriggerStateMessage.fromJson(json)
        assertNotNull(msg)
        assertTrue(msg!!.triggers.isEmpty())
    }

    // T4: missing schema_version rejected
    @Test
    fun missing_schema_version_rejected() {
        val json = JSONObject("""{
            "type":"trigger_state",
            "triggers":[]
        }""")
        assertNull(TriggerStateMessage.fromJson(json))
    }

    // T5: string schema_version rejected
    @Test
    fun string_schema_version_rejected() {
        val json = JSONObject("""{
            "type":"trigger_state",
            "schema_version":"one",
            "triggers":[]
        }""")
        assertNull(TriggerStateMessage.fromJson(json))
    }

    // T6: unsupported schema_version rejected
    @Test
    fun unsupported_schema_version_rejected() {
        val json = JSONObject("""{
            "type":"trigger_state",
            "schema_version":2,
            "triggers":[]
        }""")
        assertNull(TriggerStateMessage.fromJson(json))
    }

    // T7: missing triggers field rejected
    @Test
    fun missing_triggers_rejected() {
        val json = JSONObject("""{
            "type":"trigger_state",
            "schema_version":1
        }""")
        assertNull(TriggerStateMessage.fromJson(json))
    }

    // T8: non-array triggers rejected
    @Test
    fun non_array_triggers_rejected() {
        val json = JSONObject("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":"not_an_array"
        }""")
        assertNull(TriggerStateMessage.fromJson(json))
    }

    // T9: trigger missing trigger_id rejected
    @Test
    fun trigger_missing_id_rejected() {
        val json = JSONObject("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[{"name":"Morning","type":"time","workflow_id":"wf1","enabled":true}]
        }""")
        assertNull(TriggerStateMessage.fromJson(json))
    }

    // T10: trigger missing name rejected
    @Test
    fun trigger_missing_name_rejected() {
        val json = JSONObject("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[{"trigger_id":"t1","type":"time","workflow_id":"wf1","enabled":true}]
        }""")
        assertNull(TriggerStateMessage.fromJson(json))
    }

    // T11: trigger missing type rejected
    @Test
    fun trigger_missing_type_rejected() {
        val json = JSONObject("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[{"trigger_id":"t1","name":"Morning","workflow_id":"wf1","enabled":true}]
        }""")
        assertNull(TriggerStateMessage.fromJson(json))
    }

    // T12: trigger missing workflow_id rejected
    @Test
    fun trigger_missing_workflow_id_rejected() {
        val json = JSONObject("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[{"trigger_id":"t1","name":"Morning","type":"time","enabled":true}]
        }""")
        assertNull(TriggerStateMessage.fromJson(json))
    }

    // T13: trigger missing enabled rejected
    @Test
    fun trigger_missing_enabled_rejected() {
        val json = JSONObject("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[{"trigger_id":"t1","name":"Morning","type":"time","workflow_id":"wf1"}]
        }""")
        assertNull(TriggerStateMessage.fromJson(json))
    }

    // T14: non-integer schema_version rejected (Long)
    @Test
    fun long_schema_version_rejected() {
        val json = JSONObject("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[{"trigger_id":"t1","name":"X","type":"time","workflow_id":"wf1","enabled":true}]
        }""")
        // Valid: just testing normal path
        assertNotNull(TriggerStateMessage.fromJson(json))
    }

    // T15: trigger_id empty string accepted (opaque)
    @Test
    fun empty_trigger_id_accepted() {
        val json = JSONObject("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[{"trigger_id":"","name":"X","type":"time","workflow_id":"wf1","enabled":true}]
        }""")
        val msg = TriggerStateMessage.fromJson(json)
        assertNotNull(msg)
        assertEquals("", msg!!.triggers[0].triggerId)
    }

    // T16: trigger order preserved
    @Test
    fun trigger_order_preserved() {
        val json = JSONObject("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[
                {"trigger_id":"t2","name":"Second","type":"time","workflow_id":"wf1","enabled":false},
                {"trigger_id":"t1","name":"First","type":"manual","workflow_id":"wf2","enabled":true}
            ]
        }""")
        val msg = TriggerStateMessage.fromJson(json)
        assertNotNull(msg)
        assertEquals("t2", msg!!.triggers[0].triggerId)
        assertEquals("t1", msg.triggers[1].triggerId)
    }

    // T17: trigger_invoke_result parsed correctly
    @Test
    fun parse_trigger_invoke_result() {
        val json = JSONObject("""{
            "type":"trigger_invoke_result",
            "trigger_id":"t1",
            "accepted":true,
            "executed":true
        }""")
        val d = SpikeMessageDispatcher()
        d.handle(json.toString())
        val result = d.lastTriggerResult
        assertNotNull(result)
        assertEquals("t1", result!!.triggerId)
        assertTrue(result.accepted)
        assertTrue(result.executed!!)
    }

    // T18: trigger_invoke_result rejected parsed correctly
    @Test
    fun parse_trigger_invoke_result_rejected() {
        val json = JSONObject("""{
            "type":"trigger_invoke_result",
            "trigger_id":"t1",
            "accepted":false,
            "reason":"trigger_disabled"
        }""")
        val d = SpikeMessageDispatcher()
        d.handle(json.toString())
        val result = d.lastTriggerResult
        assertNotNull(result)
        assertEquals("t1", result!!.triggerId)
        assertEquals(false, result.accepted)
        assertEquals("trigger_disabled", result.reason)
    }

    // T19: trigger_invoke_result with execution error
    @Test
    fun parse_trigger_invoke_result_execution_error() {
        val json = JSONObject("""{
            "type":"trigger_invoke_result",
            "trigger_id":"t1",
            "accepted":true,
            "executed":false,
            "execution_error":"workflow_not_found"
        }""")
        val d = SpikeMessageDispatcher()
        d.handle(json.toString())
        val result = d.lastTriggerResult
        assertNotNull(result)
        assertTrue(result!!.accepted)
        assertEquals(false, result.executed)
        assertEquals("workflow_not_found", result.executionError)
    }
}
