package com.automatdeck.spike

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class TriggerHistoryMessageTest {

    // H1: valid single success record parsed correctly
    @Test
    fun parse_valid_single_success_record() {
        val json = JSONObject("""{
            "type":"trigger_history",
            "schema_version":1,
            "records":[{"trigger_id":"t1","workflow_id":"wf1","status":"Success","timestamp":1689600000,"duration_ms":150}]
        }""")
        val msg = TriggerHistoryMessage.fromJson(json)
        assertNotNull(msg)
        assertEquals(1, msg!!.schemaVersion)
        assertEquals(1, msg.records.size)
        assertEquals("t1", msg.records[0].triggerId)
        assertEquals("wf1", msg.records[0].workflowId)
        assertEquals(TriggerHistoryStatus.Success, msg.records[0].status)
        assertNull(msg.records[0].statusReason)
        assertEquals(1689600000L, msg.records[0].timestamp)
        assertEquals(150L, msg.records[0].durationMs)
    }

    // H2: valid failed record with reason parsed correctly
    @Test
    fun parse_valid_failed_record() {
        val json = JSONObject("""{
            "type":"trigger_history",
            "schema_version":1,
            "records":[{"trigger_id":"t1","workflow_id":"wf1","status":{"Failed":{"reason":"timeout"}},"timestamp":1000,"duration_ms":5000}]
        }""")
        val msg = TriggerHistoryMessage.fromJson(json)
        assertNotNull(msg)
        assertEquals(1, msg!!.records.size)
        assertEquals(TriggerHistoryStatus.Failed, msg.records[0].status)
        assertEquals("timeout", msg.records[0].statusReason)
    }

    // H3: valid rejected record with reason parsed correctly
    @Test
    fun parse_valid_rejected_record() {
        val json = JSONObject("""{
            "type":"trigger_history",
            "schema_version":1,
            "records":[{"trigger_id":"t1","workflow_id":"wf1","status":{"Rejected":{"reason":"workflow_disabled"}},"timestamp":1000,"duration_ms":0}]
        }""")
        val msg = TriggerHistoryMessage.fromJson(json)
        assertNotNull(msg)
        assertEquals(1, msg!!.records.size)
        assertEquals(TriggerHistoryStatus.Rejected, msg.records[0].status)
        assertEquals("workflow_disabled", msg.records[0].statusReason)
    }

    // H4: valid multiple records parsed correctly
    @Test
    fun parse_valid_multiple_records() {
        val json = JSONObject("""{
            "type":"trigger_history",
            "schema_version":1,
            "records":[
                {"trigger_id":"t1","workflow_id":"wf1","status":"Success","timestamp":1000,"duration_ms":50},
                {"trigger_id":"t2","workflow_id":"wf2","status":{"Failed":{"reason":"timeout"}},"timestamp":2000,"duration_ms":3000}
            ]
        }""")
        val msg = TriggerHistoryMessage.fromJson(json)
        assertNotNull(msg)
        assertEquals(2, msg!!.records.size)
        assertEquals("t1", msg.records[0].triggerId)
        assertEquals("t2", msg.records[1].triggerId)
    }

    // H5: empty records array parsed correctly
    @Test
    fun parse_empty_records() {
        val json = JSONObject("""{
            "type":"trigger_history",
            "schema_version":1,
            "records":[]
        }""")
        val msg = TriggerHistoryMessage.fromJson(json)
        assertNotNull(msg)
        assertTrue(msg!!.records.isEmpty())
    }

    // H6: missing schema_version rejected
    @Test
    fun missing_schema_version_rejected() {
        val json = JSONObject("""{
            "type":"trigger_history",
            "records":[]
        }""")
        assertNull(TriggerHistoryMessage.fromJson(json))
    }

    // H7: unsupported schema_version rejected
    @Test
    fun unsupported_schema_version_rejected() {
        val json = JSONObject("""{
            "type":"trigger_history",
            "schema_version":2,
            "records":[]
        }""")
        assertNull(TriggerHistoryMessage.fromJson(json))
    }

    // H8: string schema_version rejected
    @Test
    fun string_schema_version_rejected() {
        val json = JSONObject("""{
            "type":"trigger_history",
            "schema_version":"one",
            "records":[]
        }""")
        assertNull(TriggerHistoryMessage.fromJson(json))
    }

    // H9: missing records field rejected
    @Test
    fun missing_records_rejected() {
        val json = JSONObject("""{
            "type":"trigger_history",
            "schema_version":1
        }""")
        assertNull(TriggerHistoryMessage.fromJson(json))
    }

    // H10: non-array records rejected
    @Test
    fun non_array_records_rejected() {
        val json = JSONObject("""{
            "type":"trigger_history",
            "schema_version":1,
            "records":"not_an_array"
        }""")
        assertNull(TriggerHistoryMessage.fromJson(json))
    }

    // H11: record missing trigger_id still parsed (opaque)
    @Test
    fun record_missing_trigger_id_parsed() {
        val json = JSONObject("""{
            "type":"trigger_history",
            "schema_version":1,
            "records":[{"workflow_id":"wf1","status":"Success","timestamp":1000,"duration_ms":50}]
        }""")
        val msg = TriggerHistoryMessage.fromJson(json)
        assertNotNull(msg)
        assertEquals("", msg!!.records[0].triggerId)
    }

    // H12: record missing status rejected
    @Test
    fun record_missing_status_rejected() {
        val json = JSONObject("""{
            "type":"trigger_history",
            "schema_version":1,
            "records":[{"trigger_id":"t1","workflow_id":"wf1","timestamp":1000,"duration_ms":50}]
        }""")
        assertNull(TriggerHistoryMessage.fromJson(json))
    }

    // H13: trigger_history updates dispatcher
    @Test
    fun trigger_history_updates_dispatcher() {
        val d = SpikeMessageDispatcher()
        assertTrue(d.triggerHistory.isEmpty())
        d.handle("""{
            "type":"trigger_history",
            "schema_version":1,
            "records":[
                {"trigger_id":"t1","workflow_id":"wf1","status":"Success","timestamp":1000,"duration_ms":50},
                {"trigger_id":"t2","workflow_id":"wf2","status":{"Failed":{"reason":"timeout"}},"timestamp":2000,"duration_ms":3000}
            ]
        }""")
        assertEquals(2, d.triggerHistory.size)
        assertEquals("t1", d.triggerHistory[0].triggerId)
        assertEquals("t2", d.triggerHistory[1].triggerId)
    }

    // H14: second trigger_history replaces first
    @Test
    fun second_trigger_history_replaces_first() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"trigger_history",
            "schema_version":1,
            "records":[{"trigger_id":"t1","workflow_id":"wf1","status":"Success","timestamp":1000,"duration_ms":50}]
        }""")
        assertEquals(1, d.triggerHistory.size)
        d.handle("""{
            "type":"trigger_history",
            "schema_version":1,
            "records":[]
        }""")
        assertTrue(d.triggerHistory.isEmpty())
    }

    // H15: malformed trigger_history preserves existing
    @Test
    fun malformed_trigger_history_preserves_existing() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"trigger_history",
            "schema_version":1,
            "records":[{"trigger_id":"t1","workflow_id":"wf1","status":"Success","timestamp":1000,"duration_ms":50}]
        }""")
        assertEquals(1, d.triggerHistory.size)
        d.handle("not json at all")
        assertEquals(1, d.triggerHistory.size)
    }

    // H16: reset clears trigger_history
    @Test
    fun reset_clears_trigger_history() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"trigger_history",
            "schema_version":1,
            "records":[{"trigger_id":"t1","workflow_id":"wf1","status":"Success","timestamp":1000,"duration_ms":50}]
        }""")
        assertEquals(1, d.triggerHistory.size)
        d.reset()
        assertTrue(d.triggerHistory.isEmpty())
    }

    // H17: trigger_history does not mutate triggers
    @Test
    fun trigger_history_does_not_mutate_triggers() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[{"trigger_id":"t1","name":"X","type":"time","workflow_id":"wf1","enabled":true}]
        }""")
        assertEquals(1, d.triggers.size)
        d.handle("""{
            "type":"trigger_history",
            "schema_version":1,
            "records":[{"trigger_id":"t1","workflow_id":"wf1","status":"Success","timestamp":1000,"duration_ms":50}]
        }""")
        assertEquals(1, d.triggers.size)
        assertEquals(1, d.triggerHistory.size)
    }

    // H18: empty records accepted
    @Test
    fun empty_records_accepted() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"trigger_history",
            "schema_version":1,
            "records":[]
        }""")
        assertTrue(d.triggerHistory.isEmpty())
    }

    // H19: records preserve order (newest first from desktop)
    @Test
    fun records_preserve_order() {
        val json = JSONObject("""{
            "type":"trigger_history",
            "schema_version":1,
            "records":[
                {"trigger_id":"t3","workflow_id":"wf1","status":"Success","timestamp":3000,"duration_ms":30},
                {"trigger_id":"t2","workflow_id":"wf1","status":"Success","timestamp":2000,"duration_ms":20},
                {"trigger_id":"t1","workflow_id":"wf1","status":"Success","timestamp":1000,"duration_ms":10}
            ]
        }""")
        val msg = TriggerHistoryMessage.fromJson(json)
        assertNotNull(msg)
        assertEquals("t3", msg!!.records[0].triggerId)
        assertEquals("t2", msg.records[1].triggerId)
        assertEquals("t1", msg.records[2].triggerId)
    }
}
