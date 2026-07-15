package com.automatdeck.spike

import org.junit.Assert.assertEquals
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
}
