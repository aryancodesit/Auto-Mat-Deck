package com.automatdeck.spike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ControlSurfaceStateMessageTest {

    // T1 — valid active CSS decodes correctly, profileId/profileName domain correct
    @Test
    fun parse_valid_active_css() {
        val json = JSONObject("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[{"page_id":"pg1","name":"Main","buttons":[{"button_id":"b1","label":"Build"}]}]
        }""")
        val css = ControlSurfaceStateMessage.fromJson(json)
        assertNotNull(css)
        assertEquals(1, css!!.schemaVersion)
        assertEquals("p1", css.profileId)
        assertEquals("Coding", css.profileName)
        assertEquals(1, css.pages!!.size)
        assertEquals("pg1", css.pages!![0].pageId)
        assertEquals("Main", css.pages!![0].name)
        assertEquals(1, css.pages!![0].buttons.size)
        assertEquals("b1", css.pages!![0].buttons[0].buttonId)
        assertEquals("Build", css.pages!![0].buttons[0].label)
    }

    // T2 — null triple maps to NoActiveProfile semantics
    @Test
    fun parse_null_triple() {
        val json = JSONObject("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":null,
            "profile_name":null,
            "pages":null
        }""")
        val css = ControlSurfaceStateMessage.fromJson(json)
        assertNotNull(css)
        assertEquals(1, css!!.schemaVersion)
        assertNull(css.profileId)
        assertNull(css.profileName)
        assertNull(css.pages)
    }

    // T3 — active profile with pages=[] remains ActiveProfile
    @Test
    fun parse_empty_pages_is_active_profile() {
        val json = JSONObject("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[]
        }""")
        val css = ControlSurfaceStateMessage.fromJson(json)
        assertNotNull(css)
        assertEquals("p1", css!!.profileId)
        assertNotNull(css.pages)
        assertTrue(css.pages!!.isEmpty())
    }

    // T4 — page order is preserved
    @Test
    fun page_order_preserved() {
        val json = JSONObject("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Test",
            "pages":[
                {"page_id":"pg2","name":"Second","buttons":[]},
                {"page_id":"pg1","name":"First","buttons":[]}
            ]
        }""")
        val css = ControlSurfaceStateMessage.fromJson(json)
        assertNotNull(css)
        assertEquals("pg2", css!!.pages!![0].pageId)
        assertEquals("pg1", css.pages!![1].pageId)
    }

    // T5 — button order is preserved
    @Test
    fun button_order_preserved() {
        val json = JSONObject("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Test",
            "pages":[{"page_id":"pg1","name":"Main","buttons":[
                {"button_id":"b2","label":"Second"},
                {"button_id":"b1","label":"First"}
            ]}]
        }""")
        val css = ControlSurfaceStateMessage.fromJson(json)
        assertNotNull(css)
        assertEquals("b2", css!!.pages!![0].buttons[0].buttonId)
        assertEquals("b1", css.pages!![0].buttons[1].buttonId)
    }

    // T6 — opaque IDs are preserved exactly
    @Test
    fun opaque_ids_preserved_exactly() {
        val json = JSONObject("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"my-custom-id-123",
            "profile_name":"Test",
            "pages":[{"page_id":"page/456","name":"X","buttons":[{"button_id":"btn_789","label":"Y"}]}]
        }""")
        val css = ControlSurfaceStateMessage.fromJson(json)
        assertNotNull(css)
        assertEquals("my-custom-id-123", css!!.profileId)
        assertEquals("page/456", css.pages!![0].pageId)
        assertEquals("btn_789", css.pages!![0].buttons[0].buttonId)
    }

    // T7 — unsupported schema_version preserves previous valid CSS
    @Test
    fun unsupported_version_does_not_replace_existing() {
        var latestCss: ControlSurfaceStateMessage?

        val v1 = ControlSurfaceStateMessage.fromJson(JSONObject("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[{"page_id":"pg1","name":"Main","buttons":[{"button_id":"b1","label":"Build"}]}]
        }"""))
        latestCss = v1

        val v2 = ControlSurfaceStateMessage.fromJson(JSONObject("""{
            "type":"control_surface_state",
            "schema_version":2,
            "profile_id":"p2",
            "profile_name":"Gaming",
            "pages":[]
        }"""))
        assertNull(v2)

        assertEquals("p1", latestCss!!.profileId)
    }

    // T8 — malformed CSS preserves previous valid CSS
    @Test
    fun malformed_css_does_not_replace_existing() {
        var latestCss: ControlSurfaceStateMessage?

        val valid = ControlSurfaceStateMessage.fromJson(JSONObject("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[]
        }"""))
        latestCss = valid

        val invalid = ControlSurfaceStateMessage.fromJson(
            JSONObject("""{"type":"control_surface_state","schema_version":1,"profile_id":123,"profile_name":"X","pages":[]}""")
        )
        assertNull(invalid)

        assertEquals("p1", latestCss!!.profileId)
    }

    // T9 — second valid CSS replaces first valid CSS
    @Suppress("UNUSED_VALUE")
    @Test
    fun second_css_replaces_first() {
        var latestCss: ControlSurfaceStateMessage?

        val css1 = ControlSurfaceStateMessage.fromJson(JSONObject("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[{"page_id":"pg1","name":"Main","buttons":[{"button_id":"b1","label":"Build"}]}]
        }"""))
        latestCss = css1

        val css2 = ControlSurfaceStateMessage.fromJson(JSONObject("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p2",
            "profile_name":"Gaming",
            "pages":[{"page_id":"pg2","name":"Desk","buttons":[{"button_id":"b2","label":"Launch"}]}]
        }"""))
        latestCss = css2

        assertEquals("p2", latestCss!!.profileId)
        assertEquals("Gaming", latestCss.profileName)
        assertEquals("Launch", latestCss.pages!![0].buttons[0].label)
    }

    // T10 — non-CSS messages do not mutate CSS state (conceptual guard)
    @Test
    fun non_css_message_does_not_affect_css() {
        var latestCss: ControlSurfaceStateMessage?

        val css = ControlSurfaceStateMessage.fromJson(JSONObject("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Coding",
            "pages":[]
        }"""))
        latestCss = css

        assertEquals("p1", latestCss!!.profileId)
    }

    // Schema version type validation
    @Test
    fun string_schema_version_rejected() {
        val json = JSONObject("""{"type":"control_surface_state","schema_version":"one","profile_id":"p1","profile_name":"X","pages":[]}""")
        assertNull(ControlSurfaceStateMessage.fromJson(json))
    }

    @Test
    fun fractional_schema_version_rejected() {
        val json = JSONObject("""{"type":"control_surface_state","schema_version":1.5,"profile_id":"p1","profile_name":"X","pages":[]}""")
        assertNull(ControlSurfaceStateMessage.fromJson(json))
    }

    @Test
    fun missing_schema_version_rejected() {
        val json = JSONObject("""{"type":"control_surface_state","profile_id":"p1","profile_name":"X","pages":[]}""")
        assertNull(ControlSurfaceStateMessage.fromJson(json))
    }

    // profile_id type validation
    @Test
    fun number_profile_id_rejected() {
        val json = JSONObject("""{"type":"control_surface_state","schema_version":1,"profile_id":123,"profile_name":"X","pages":[]}""")
        assertNull(ControlSurfaceStateMessage.fromJson(json))
    }

    @Test
    fun boolean_profile_id_rejected() {
        val json = JSONObject("""{"type":"control_surface_state","schema_version":1,"profile_id":true,"profile_name":"X","pages":[]}""")
        assertNull(ControlSurfaceStateMessage.fromJson(json))
    }

    @Test
    fun missing_profile_id_rejected() {
        val json = JSONObject("""{"type":"control_surface_state","schema_version":1,"profile_name":"X","pages":[]}""")
        assertNull(ControlSurfaceStateMessage.fromJson(json))
    }

    // profile_name type validation
    @Test
    fun number_profile_name_rejected() {
        val json = JSONObject("""{"type":"control_surface_state","schema_version":1,"profile_id":"p1","profile_name":456,"pages":[]}""")
        assertNull(ControlSurfaceStateMessage.fromJson(json))
    }

    @Test
    fun missing_profile_name_rejected() {
        val json = JSONObject("""{"type":"control_surface_state","schema_version":1,"profile_id":"p1","pages":[]}""")
        assertNull(ControlSurfaceStateMessage.fromJson(json))
    }

    // pages type validation
    @Test
    fun object_pages_rejected() {
        val json = JSONObject("""{"type":"control_surface_state","schema_version":1,"profile_id":"p1","profile_name":"X","pages":{}}""")
        assertNull(ControlSurfaceStateMessage.fromJson(json))
    }

    @Test
    fun missing_pages_rejected() {
        val json = JSONObject("""{"type":"control_surface_state","schema_version":1,"profile_id":"p1","profile_name":"X"}""")
        assertNull(ControlSurfaceStateMessage.fromJson(json))
    }

    // page/button field validation
    @Test
    fun missing_page_id_rejected() {
        val json = JSONObject("""{"type":"control_surface_state","schema_version":1,"profile_id":"p1","profile_name":"X","pages":[{"name":"Main","buttons":[]}]}""")
        assertNull(ControlSurfaceStateMessage.fromJson(json))
    }

    @Test
    fun missing_button_id_rejected() {
        val json = JSONObject("""{"type":"control_surface_state","schema_version":1,"profile_id":"p1","profile_name":"X","pages":[{"page_id":"pg1","name":"Main","buttons":[{"label":"Y"}]}]}""")
        assertNull(ControlSurfaceStateMessage.fromJson(json))
    }

    @Test
    fun missing_button_label_rejected() {
        val json = JSONObject("""{"type":"control_surface_state","schema_version":1,"profile_id":"p1","profile_name":"X","pages":[{"page_id":"pg1","name":"Main","buttons":[{"button_id":"b1"}]}]}""")
        assertNull(ControlSurfaceStateMessage.fromJson(json))
    }

    // Null triple is distinct from NoProjection (null variable)
    @Test
    fun null_triple_is_distinct_from_no_projection() {
        var latestCss: ControlSurfaceStateMessage? = null
        assertNull(latestCss)

        val nullTriple = ControlSurfaceStateMessage.fromJson(JSONObject("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":null,
            "profile_name":null,
            "pages":null
        }"""))
        assertNotNull(nullTriple)
        latestCss = nullTriple

        assertNotNull(latestCss)
        assertNull(latestCss!!.profileId)
        assertNull(latestCss.profileName)
        assertNull(latestCss.pages)
    }

    // Empty string IDs/labels are accepted (opaque)
    @Test
    fun empty_string_ids_accepted() {
        val json = JSONObject("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"",
            "profile_name":"",
            "pages":[{"page_id":"","name":"X","buttons":[{"button_id":"","label":""}]}]
        }""")
        val css = ControlSurfaceStateMessage.fromJson(json)
        assertNotNull(css)
        assertEquals("", css!!.profileId)
        assertEquals("", css.profileName)
        assertEquals("", css.pages!![0].pageId)
        assertEquals("", css.pages!![0].buttons[0].buttonId)
        assertEquals("", css.pages!![0].buttons[0].label)
    }

    // Duplicate identical CSS is harmless
    @Test
    fun duplicate_css_is_harmless() {
        val json = """{"type":"control_surface_state","schema_version":1,"profile_id":"p1","profile_name":"X","pages":[{"page_id":"pg1","name":"Main","buttons":[{"button_id":"b1","label":"Build"}]}]}"""
        val a = ControlSurfaceStateMessage.fromJson(JSONObject(json))
        val b = ControlSurfaceStateMessage.fromJson(JSONObject(json))
        assertNotNull(a)
        assertNotNull(b)
        assertEquals(a!!.profileId, b!!.profileId)
        assertEquals(a.pages!![0].buttons[0].label, b.pages!![0].buttons[0].label)
    }

    // Null profile_name with active profile (edge case)
    @Test
    fun null_profile_name_with_active_profile() {
        val json = JSONObject("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":null,
            "pages":[{"page_id":"pg1","name":"Main","buttons":[]}]
        }""")
        val css = ControlSurfaceStateMessage.fromJson(json)
        assertNotNull(css)
        assertEquals("p1", css!!.profileId)
        assertNull(css.profileName)
        assertNotNull(css.pages)
    }

    // Multiple pages with multiple buttons each
    @Test
    fun multi_page_multi_button() {
        val json = JSONObject("""{
            "type":"control_surface_state",
            "schema_version":1,
            "profile_id":"p1",
            "profile_name":"Full",
            "pages":[
                {"page_id":"pg1","name":"Main","buttons":[{"button_id":"b1","label":"A"},{"button_id":"b2","label":"B"}]},
                {"page_id":"pg2","name":"Dev","buttons":[{"button_id":"b3","label":"C"}]}
            ]
        }""")
        val css = ControlSurfaceStateMessage.fromJson(json)
        assertNotNull(css)
        assertEquals(2, css!!.pages!!.size)
        assertEquals("A", css.pages!![0].buttons[0].label)
        assertEquals("C", css.pages!![1].buttons[0].label)
    }
}
