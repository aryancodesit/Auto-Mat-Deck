package com.automatdeck.spike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class SpikeMessageDispatcherTest {

    // B1: initial state is NoProjection
    @Test
    fun initial_state_is_no_projection() {
        val d = SpikeMessageDispatcher()
        assertTrue(d.uiState is ControlSurfaceUiState.NoProjection)
    }

    // B2: exact null triple -> NoActiveProfile
    @Test
    fun exact_null_triple_maps_to_no_active_profile() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":null,
            "profile_name":null,
            "pages":null
        }""")
        assertTrue(d.uiState is ControlSurfaceUiState.NoActiveProfile)
    }

    // B3: valid non-null triple -> ActiveProfile
    @Test
    fun valid_non_null_triple_maps_to_active_profile() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[{"page_id":"pg1","name":"Main","buttons":[{"button_id":"b1","label":"Build"}]}]
        }""")
        val state = d.uiState as? ControlSurfaceUiState.ActiveProfile
        assertTrue(state != null)
        assertEquals("p1", state!!.profileId)
        assertEquals("Coding", state.profileName)
        assertEquals(1, state.pages.size)
    }

    // B4: pages=[] remains valid ActiveProfile
    @Test
    fun empty_pages_is_valid_active_profile() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[]
        }""")
        val state = d.uiState as? ControlSurfaceUiState.ActiveProfile
        assertTrue(state != null)
        assertTrue(state!!.pages.isEmpty())
    }

    // B5: mixed-null A (null "Coding" []) preserves previous valid state
    @Test
    fun mixed_null_a_preserves_previous_state() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[{"page_id":"pg1","name":"Main","buttons":[{"button_id":"b1","label":"Build"}]}]
        }""")
        assertTrue(d.uiState is ControlSurfaceUiState.ActiveProfile)

        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":null,
            "profile_name":"Coding",
            "pages":[]
        }""")
        val state = d.uiState as? ControlSurfaceUiState.ActiveProfile
        assertTrue(state != null)
        assertEquals("p1", state!!.profileId)
    }

    // B6: mixed-null B ("p1" null []) preserves previous valid state
    @Test
    fun mixed_null_b_preserves_previous_state() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[{"page_id":"pg1","name":"Main","buttons":[]}]
        }""")
        assertTrue(d.uiState is ControlSurfaceUiState.ActiveProfile)

        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":null,
            "pages":[]
        }""")
        val state = d.uiState as? ControlSurfaceUiState.ActiveProfile
        assertTrue(state != null)
        assertEquals("p1", state!!.profileId)
    }

    // B7: mixed-null C ("p1" "Coding" null) preserves previous valid state
    @Test
    fun mixed_null_c_preserves_previous_state() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[{"page_id":"pg1","name":"Main","buttons":[]}]
        }""")
        assertTrue(d.uiState is ControlSurfaceUiState.ActiveProfile)

        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":null
        }""")
        val state = d.uiState as? ControlSurfaceUiState.ActiveProfile
        assertTrue(state != null)
        assertEquals("p1", state!!.profileId)
    }

    // B8: mixed-null (null null []) preserves previous valid state
    @Test
    fun mixed_null_null_array_preserves_previous_state() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[]
        }""")
        assertTrue(d.uiState is ControlSurfaceUiState.ActiveProfile)

        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":null,
            "profile_name":null,
            "pages":[]
        }""")
        assertTrue(d.uiState is ControlSurfaceUiState.ActiveProfile)
    }

    // B9: mixed-null ("p1" null null) preserves previous valid state
    @Test
    fun mixed_null_p1_null_null_preserves_previous_state() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[]
        }""")
        assertTrue(d.uiState is ControlSurfaceUiState.ActiveProfile)

        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":null,
            "pages":null
        }""")
        assertTrue(d.uiState is ControlSurfaceUiState.ActiveProfile)
    }

    // B10: mixed-null (null "Coding" null) preserves previous valid state
    @Test
    fun mixed_null_null_coding_null_preserves_previous_state() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[]
        }""")
        assertTrue(d.uiState is ControlSurfaceUiState.ActiveProfile)

        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":null,
            "profile_name":"Coding",
            "pages":null
        }""")
        assertTrue(d.uiState is ControlSurfaceUiState.ActiveProfile)
    }

    // B11: second valid CSS replaces first
    @Test
    fun second_valid_css_replaces_first() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[]
        }""")
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p2",
            "profile_name":"Gaming",
            "pages":[]
        }""")
        val state = d.uiState as ControlSurfaceUiState.ActiveProfile
        assertEquals("p2", state.profileId)
        assertEquals("Gaming", state.profileName)
    }

    // B12: unsupported schema version preserves previous valid state
    @Test
    fun unsupported_version_preserves_previous_state() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[]
        }""")
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":2,
            "profile_id":"p2",
            "profile_name":"Gaming",
            "pages":[]
        }""")
        val state = d.uiState as ControlSurfaceUiState.ActiveProfile
        assertEquals("p1", state.profileId)
    }

    // B13: malformed JSON preserves previous valid state
    @Test
    fun malformed_json_preserves_previous_state() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[]
        }""")
        d.handle("not json at all")
        val state = d.uiState as ControlSurfaceUiState.ActiveProfile
        assertEquals("p1", state.profileId)
    }

    // B14: unrelated message type preserves previous valid state
    @Test
    fun unrelated_message_type_preserves_state() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[]
        }""")
        d.handle("""{"type":"ping","timestamp":1234}""")
        val state = d.uiState as ControlSurfaceUiState.ActiveProfile
        assertEquals("p1", state.profileId)
    }

    // B15: page order survives production dispatch
    @Test
    fun page_order_preserved_through_dispatch() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Test",
            "pages":[
                {"page_id":"pg2","name":"Second","buttons":[]},
                {"page_id":"pg1","name":"First","buttons":[]}
            ]
        }""")
        val state = d.uiState as ControlSurfaceUiState.ActiveProfile
        assertEquals(2, state.pages.size)
        assertEquals("pg2", state.pages[0].pageId)
        assertEquals("pg1", state.pages[1].pageId)
    }

    // B16: button order survives production dispatch
    @Test
    fun button_order_preserved_through_dispatch() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Test",
            "pages":[{"page_id":"pg1","name":"Main","buttons":[
                {"button_id":"b2","label":"Second"},
                {"button_id":"b1","label":"First"}
            ]}]
        }""")
        val state = d.uiState as ControlSurfaceUiState.ActiveProfile
        assertEquals("b2", state.pages[0].buttons[0].buttonId)
        assertEquals("b1", state.pages[0].buttons[1].buttonId)
    }

    // B17: opaque profile_id survives exactly
    @Test
    fun opaque_profile_id_survives_dispatch() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"my-custom-id-123",
            "profile_name":"Test",
            "pages":[]
        }""")
        val state = d.uiState as ControlSurfaceUiState.ActiveProfile
        assertEquals("my-custom-id-123", state.profileId)
    }

    // B18: opaque page_id survives exactly
    @Test
    fun opaque_page_id_survives_dispatch() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Test",
            "pages":[{"page_id":"page/456","name":"X","buttons":[]}]
        }""")
        val state = d.uiState as ControlSurfaceUiState.ActiveProfile
        assertEquals("page/456", state.pages[0].pageId)
    }

    // B19: opaque button_id survives exactly
    @Test
    fun opaque_button_id_survives_dispatch() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Test",
            "pages":[{"page_id":"pg1","name":"X","buttons":[{"button_id":"btn_789","label":"Y"}]}]
        }""")
        val state = d.uiState as ControlSurfaceUiState.ActiveProfile
        assertEquals("btn_789", state.pages[0].buttons[0].buttonId)
    }

    // B20: null APS -> NoProjection
    @Test
    fun null_aps_goes_to_no_projection() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[]
        }""")
        assertTrue(d.uiState is ControlSurfaceUiState.ActiveProfile)

        d.handle("""{"type":"active_profile_state","schema_version":1,"active_profile_id":null}""")
        assertTrue(d.uiState is ControlSurfaceUiState.NoProjection)
        assertNull(d.lastRaw)
    }

    // B21: exact null triple from NoProjection stays NoProjection (no prior active)
    @Test
    fun null_triple_from_no_projection_stays_no_projection() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":null,
            "profile_name":null,
            "pages":null
        }""")
        assertTrue(d.uiState is ControlSurfaceUiState.NoActiveProfile)
    }

    // B22: reset
    @Test
    fun reset_clears_state() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[]
        }""")
        assertTrue(d.uiState is ControlSurfaceUiState.ActiveProfile)
        d.reset()
        assertTrue(d.uiState is ControlSurfaceUiState.NoProjection)
        assertNull(d.lastRaw)
    }

    // ── Path F: control_invoke_result ──

    // F1: accepted response parsed correctly
    @Test
    fun invoke_result_accepted_parsed() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_invoke_result",
            "schema_version":1,
            "button_id":"wifi",
            "accepted":true
        }""")
        val result = d.lastInvokeResult
        assertTrue(result != null)
        assertEquals("wifi", result!!.buttonId)
        assertTrue(result.accepted)
        assertNull(result.reason)
    }

    // F2: rejected response with reason parsed correctly
    @Test
    fun invoke_result_rejected_parsed() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_invoke_result",
            "schema_version":1,
            "button_id":"wifi",
            "accepted":false,
            "reason":"unknown_button"
        }""")
        val result = d.lastInvokeResult
        assertTrue(result != null)
        assertFalse(result!!.accepted)
        assertEquals("unknown_button", result.reason)
    }

    // F3: no_active_profile rejection
    @Test
    fun invoke_result_no_active_profile_parsed() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_invoke_result",
            "schema_version":1,
            "button_id":"wifi",
            "accepted":false,
            "reason":"no_active_profile"
        }""")
        val result = d.lastInvokeResult
        assertTrue(result != null)
        assertEquals("no_active_profile", result!!.reason)
    }

    // F4: ambiguous_button rejection
    @Test
    fun invoke_result_ambiguous_button_parsed() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_invoke_result",
            "schema_version":1,
            "button_id":"wifi",
            "accepted":false,
            "reason":"ambiguous_button"
        }""")
        val result = d.lastInvokeResult
        assertTrue(result != null)
        assertEquals("ambiguous_button", result!!.reason)
    }

    // F5: control_invoke_result does not mutate ControlSurfaceUiState
    @Test
    fun invoke_result_does_not_mutate_ui_state() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[]
        }""")
        val stateBefore = d.uiState

        d.handle("""{
            "type":"control_invoke_result",
            "schema_version":1,
            "button_id":"wifi",
            "accepted":false,
            "reason":"unknown_button"
        }""")

        assertTrue(d.uiState === stateBefore)
    }

    // F6: reset clears lastInvokeResult
    @Test
    fun reset_clears_last_invoke_result() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_invoke_result",
            "schema_version":1,
            "button_id":"wifi",
            "accepted":true
        }""")
        assertTrue(d.lastInvokeResult != null)
        d.reset()
        assertNull(d.lastInvokeResult)
    }

    // F7: unrelated message type preserves lastInvokeResult
    @Test
    fun unrelated_message_preserves_invoke_result() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_invoke_result",
            "schema_version":1,
            "button_id":"wifi",
            "accepted":true,
            "executed":true
        }""")
        d.handle("""{"type":"ping","timestamp":1234}""")
        assertTrue(d.lastInvokeResult != null)
        assertEquals("wifi", d.lastInvokeResult!!.buttonId)
    }

    // Path G: Sprint 4 — accepted with executed=true
    @Test
    fun invoke_result_accepted_executed() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_invoke_result",
            "schema_version":1,
            "button_id":"wifi",
            "accepted":true,
            "executed":true
        }""")
        val r = d.lastInvokeResult!!
        assertTrue(r.accepted)
        assertTrue(r.executed!!)
        assertNull(r.reason)
        assertNull(r.executionError)
    }

    // Path G: Sprint 4 — accepted but execution failed
    @Test
    fun invoke_result_accepted_execution_failed() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_invoke_result",
            "schema_version":1,
            "button_id":"wifi",
            "accepted":true,
            "executed":false,
            "execution_error":"execution_timeout"
        }""")
        val r = d.lastInvokeResult!!
        assertTrue(r.accepted)
        assertEquals(false, r.executed)
        assertEquals("execution_timeout", r.executionError)
        assertNull(r.reason)
    }

    // Path G: Sprint 4 — rejected (no executed/execution_error)
    @Test
    fun invoke_result_rejected_no_execution_fields() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_invoke_result",
            "schema_version":1,
            "button_id":"wifi",
            "accepted":false,
            "reason":"no_active_profile"
        }""")
        val r = d.lastInvokeResult!!
        assertFalse(r.accepted)
        assertNull(r.executed)
        assertNull(r.executionError)
        assertEquals("no_active_profile", r.reason)
    }

    // Path G: Sprint 4 — reset clears execution fields too
    @Test
    fun reset_clears_execution_fields() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_invoke_result",
            "schema_version":1,
            "button_id":"wifi",
            "accepted":true,
            "executed":true
        }""")
        assertTrue(d.lastInvokeResult?.executed != null)
        d.reset()
        assertNull(d.lastInvokeResult)
    }

    // ── Path H: Sprint 3 — workflow steps ──

    // H1: v0.5 response (no steps field) → empty steps list
    @Test
    fun invoke_result_v05_no_steps_field() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_invoke_result",
            "schema_version":1,
            "button_id":"wifi",
            "accepted":true,
            "executed":true
        }""")
        val r = d.lastInvokeResult!!
        assertTrue(r.accepted)
        assertTrue(r.executed!!)
        assertTrue(r.steps.isEmpty())
    }

    // H2: v0.5 rejected response (no steps) → empty steps list
    @Test
    fun invoke_result_v05_rejected_no_steps() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_invoke_result",
            "schema_version":1,
            "button_id":"wifi",
            "accepted":false,
            "reason":"unknown_button"
        }""")
        val r = d.lastInvokeResult!!
        assertFalse(r.accepted)
        assertTrue(r.steps.isEmpty())
    }

    // H3: v0.6 workflow response with steps
    @Test
    fun invoke_result_v06_workflow_with_steps() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_invoke_result",
            "schema_version":1,
            "button_id":"wf-btn",
            "accepted":true,
            "executed":true,
            "steps":[
                {"step_index":0,"action_id":"lock","executed":true},
                {"step_index":1,"action_id":"launch","executed":true}
            ]
        }""")
        val r = d.lastInvokeResult!!
        assertTrue(r.accepted)
        assertTrue(r.executed!!)
        assertEquals(2, r.steps.size)
        assertEquals(0, r.steps[0].stepIndex)
        assertEquals("lock", r.steps[0].actionId)
        assertTrue(r.steps[0].executed)
        assertNull(r.steps[0].error)
        assertEquals(1, r.steps[1].stepIndex)
        assertEquals("launch", r.steps[1].actionId)
        assertTrue(r.steps[1].executed)
    }

    // H4: v0.6 workflow with step failure
    @Test
    fun invoke_result_v06_workflow_step_failure() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_invoke_result",
            "schema_version":1,
            "button_id":"wf-btn",
            "accepted":true,
            "executed":false,
            "execution_error":"action_not_found",
            "steps":[
                {"step_index":0,"action_id":"lock","executed":true},
                {"step_index":1,"action_id":"nonexistent","executed":false,"error":"action_not_found"}
            ]
        }""")
        val r = d.lastInvokeResult!!
        assertTrue(r.accepted)
        assertEquals(false, r.executed)
        assertEquals("action_not_found", r.executionError)
        assertEquals(2, r.steps.size)
        assertTrue(r.steps[0].executed)
        assertNull(r.steps[0].error)
        assertFalse(r.steps[1].executed)
        assertEquals("action_not_found", r.steps[1].error)
    }

    // H5: v0.6 action response with empty steps array
    @Test
    fun invoke_result_v06_action_empty_steps() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_invoke_result",
            "schema_version":1,
            "button_id":"wifi",
            "accepted":true,
            "executed":true,
            "steps":[]
        }""")
        val r = d.lastInvokeResult!!
        assertTrue(r.accepted)
        assertTrue(r.executed!!)
        assertTrue(r.steps.isEmpty())
    }

    // H6: v0.6 workflow disabled rejection (no steps)
    @Test
    fun invoke_result_v06_workflow_disabled_no_steps() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_invoke_result",
            "schema_version":1,
            "button_id":"wf-btn",
            "accepted":false,
            "reason":"workflow_disabled"
        }""")
        val r = d.lastInvokeResult!!
        assertFalse(r.accepted)
        assertEquals("workflow_disabled", r.reason)
        assertTrue(r.steps.isEmpty())
    }

    // H7: step with error field present
    @Test
    fun invoke_result_step_with_error() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_invoke_result",
            "schema_version":1,
            "button_id":"wf-btn",
            "accepted":true,
            "executed":false,
            "steps":[{"step_index":0,"action_id":"bad","executed":false,"error":"execution_timeout"}]
        }""")
        val r = d.lastInvokeResult!!
        assertEquals(1, r.steps.size)
        assertEquals("execution_timeout", r.steps[0].error)
    }

    // H8: step without error field → null
    @Test
    fun invoke_result_step_without_error_is_null() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_invoke_result",
            "schema_version":1,
            "button_id":"wf-btn",
            "accepted":true,
            "executed":true,
            "steps":[{"step_index":0,"action_id":"lock","executed":true}]
        }""")
        val r = d.lastInvokeResult!!
        assertNull(r.steps[0].error)
    }

    // H9: reset clears steps
    @Test
    fun reset_clears_steps() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_invoke_result",
            "schema_version":1,
            "button_id":"wf-btn",
            "accepted":true,
            "executed":true,
            "steps":[{"step_index":0,"action_id":"lock","executed":true}]
        }""")
        assertEquals(1, d.lastInvokeResult!!.steps.size)
        d.reset()
        assertNull(d.lastInvokeResult)
    }

    // ── Path I: trigger_state ──

    // I1: valid trigger_state updates triggers
    @Test
    fun trigger_state_updates_triggers() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[
                {"trigger_id":"t1","name":"Morning","type":"time","workflow_id":"wf1","enabled":true},
                {"trigger_id":"t2","name":"Manual","type":"manual","workflow_id":"wf2","enabled":false}
            ]
        }""")
        assertEquals(2, d.triggers.size)
        assertEquals("t1", d.triggers[0].triggerId)
        assertEquals("Morning", d.triggers[0].name)
        assertTrue(d.triggers[0].enabled)
        assertEquals("t2", d.triggers[1].triggerId)
        assertEquals(false, d.triggers[1].enabled)
    }

    // I2: empty trigger_state clears triggers
    @Test
    fun trigger_state_empty_triggers() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[
                {"trigger_id":"t1","name":"X","type":"time","workflow_id":"wf1","enabled":true}
            ]
        }""")
        assertEquals(1, d.triggers.size)
        d.handle("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[]
        }""")
        assertTrue(d.triggers.isEmpty())
    }

    // I3: malformed trigger_state preserves existing triggers
    @Test
    fun malformed_trigger_state_preserves_existing() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[{"trigger_id":"t1","name":"X","type":"time","workflow_id":"wf1","enabled":true}]
        }""")
        assertEquals(1, d.triggers.size)
        d.handle("not json at all")
        assertEquals(1, d.triggers.size)
    }

    // I4: trigger_state does not mutate ControlSurfaceUiState
    @Test
    fun trigger_state_does_not_mutate_ui_state() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[]
        }""")
        val stateBefore = d.uiState
        d.handle("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[{"trigger_id":"t1","name":"X","type":"time","workflow_id":"wf1","enabled":true}]
        }""")
        assertTrue(d.uiState === stateBefore)
    }

    // I5: unrelated message preserves triggers
    @Test
    fun unrelated_message_preserves_triggers() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[{"trigger_id":"t1","name":"X","type":"time","workflow_id":"wf1","enabled":true}]
        }""")
        d.handle("""{"type":"ping","timestamp":1234}""")
        assertEquals(1, d.triggers.size)
    }

    // I6: second trigger_state replaces first
    @Test
    fun second_trigger_state_replaces_first() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[{"trigger_id":"t1","name":"First","type":"time","workflow_id":"wf1","enabled":true}]
        }""")
        d.handle("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[{"trigger_id":"t2","name":"Second","type":"manual","workflow_id":"wf2","enabled":false}]
        }""")
        assertEquals(1, d.triggers.size)
        assertEquals("t2", d.triggers[0].triggerId)
    }

    // I7: reset clears triggers
    @Test
    fun reset_clears_triggers() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[{"trigger_id":"t1","name":"X","type":"time","workflow_id":"wf1","enabled":true}]
        }""")
        assertEquals(1, d.triggers.size)
        d.reset()
        assertTrue(d.triggers.isEmpty())
    }

    // I8: initial triggers is empty
    @Test
    fun initial_triggers_is_empty() {
        val d = SpikeMessageDispatcher()
        assertTrue(d.triggers.isEmpty())
    }

    // I9: initial lastTriggerResult is null
    @Test
    fun initial_last_trigger_result_is_null() {
        val d = SpikeMessageDispatcher()
        assertNull(d.lastTriggerResult)
    }

    // ── Path J: trigger_invoke_result ──

    // J1: accepted trigger result parsed correctly
    @Test
    fun trigger_invoke_result_accepted() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"trigger_invoke_result",
            "trigger_id":"t1",
            "accepted":true,
            "executed":true
        }""")
        val r = d.lastTriggerResult
        assertNotNull(r)
        assertEquals("t1", r!!.triggerId)
        assertTrue(r.accepted)
        assertTrue(r.executed!!)
    }

    // J2: rejected trigger result parsed correctly
    @Test
    fun trigger_invoke_result_rejected() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"trigger_invoke_result",
            "trigger_id":"t1",
            "accepted":false,
            "reason":"trigger_disabled"
        }""")
        val r = d.lastTriggerResult
        assertNotNull(r)
        assertFalse(r!!.accepted)
        assertEquals("trigger_disabled", r.reason)
    }

    // J3: trigger result with execution error
    @Test
    fun trigger_invoke_result_execution_error() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"trigger_invoke_result",
            "trigger_id":"t1",
            "accepted":true,
            "executed":false,
            "execution_error":"workflow_not_found"
        }""")
        val r = d.lastTriggerResult
        assertNotNull(r)
        assertTrue(r!!.accepted)
        assertFalse(r.executed!!)
        assertEquals("workflow_not_found", r.executionError)
    }

    // J4: trigger_invoke_result does not mutate UI state
    @Test
    fun trigger_invoke_result_does_not_mutate_ui_state() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[]
        }""")
        val stateBefore = d.uiState
        d.handle("""{
            "type":"trigger_invoke_result",
            "trigger_id":"t1",
            "accepted":true,
            "executed":true
        }""")
        assertTrue(d.uiState === stateBefore)
    }

    // J5: trigger_invoke_result does not mutate triggers
    @Test
    fun trigger_invoke_result_does_not_mutate_triggers() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"trigger_state",
            "schema_version":1,
            "triggers":[{"trigger_id":"t1","name":"X","type":"time","workflow_id":"wf1","enabled":true}]
        }""")
        assertEquals(1, d.triggers.size)
        d.handle("""{
            "type":"trigger_invoke_result",
            "trigger_id":"t1",
            "accepted":true,
            "executed":true
        }""")
        assertEquals(1, d.triggers.size)
    }

    // J6: reset clears lastTriggerResult
    @Test
    fun reset_clears_last_trigger_result() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"trigger_invoke_result",
            "trigger_id":"t1",
            "accepted":true,
            "executed":true
        }""")
        assertNotNull(d.lastTriggerResult)
        d.reset()
        assertNull(d.lastTriggerResult)
    }

    // J7: empty trigger_id rejected
    @Test
    fun trigger_invoke_result_empty_id_rejected() {
        val d = SpikeMessageDispatcher()
        d.handle("""{
            "type":"trigger_invoke_result",
            "trigger_id":"",
            "accepted":true,
            "executed":true
        }""")
        assertNull(d.lastTriggerResult)
    }
}
