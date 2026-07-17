package com.automatdeck.spike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ControlSurfacePresentationMapperTest {

    // C1: ActiveProfile emits profile header with exact profile name
    @Test
    fun active_profile_emits_profile_header() {
        val state = ControlSurfaceUiState.ActiveProfile(
            profileId = "p1",
            profileName = "Coding",
            pages = listOf(PageMessage("pg1", "Main", listOf(ButtonMessage("b1", "Build"))))
        )
        val items = ControlSurfacePresentationMapper.map(state)
        val header = items[0] as? ControlSurfacePresentationItem.ProfileHeader
        assertTrue(header != null)
        assertEquals("Coding", header!!.profileName)
    }

    // C2: profile ID is retained exactly
    @Test
    fun profile_id_retained_exactly() {
        val state = ControlSurfaceUiState.ActiveProfile(
            profileId = "my-custom-id-123",
            profileName = "Test",
            pages = emptyList()
        )
        val items = ControlSurfacePresentationMapper.map(state)
        val header = items[0] as ControlSurfacePresentationItem.ProfileHeader
        assertEquals("my-custom-id-123", header.profileId)
    }

    // C3: all pages are emitted
    @Test
    fun all_pages_are_emitted() {
        val state = ControlSurfaceUiState.ActiveProfile(
            profileId = "p1",
            profileName = "Test",
            pages = listOf(
                PageMessage("pg1", "Main", listOf(ButtonMessage("b1", "A"))),
                PageMessage("pg2", "Git", listOf(ButtonMessage("b2", "B"))),
                PageMessage("pg3", "Tools", listOf(ButtonMessage("b3", "C"))),
            )
        )
        val items = ControlSurfacePresentationMapper.map(state)
        val pageHeaders = items.filterIsInstance<ControlSurfacePresentationItem.PageHeader>()
        assertEquals(3, pageHeaders.size)
    }

    // C4: page order is preserved
    @Test
    fun page_order_preserved() {
        val state = ControlSurfaceUiState.ActiveProfile(
            profileId = "p1",
            profileName = "Test",
            pages = listOf(
                PageMessage("pg2", "Second", emptyList()),
                PageMessage("pg1", "First", emptyList()),
            )
        )
        val items = ControlSurfacePresentationMapper.map(state)
        val pageHeaders = items.filterIsInstance<ControlSurfacePresentationItem.PageHeader>()
        assertEquals("pg2", pageHeaders[0].pageId)
        assertEquals("pg1", pageHeaders[1].pageId)
    }

    // C5: page names are retained
    @Test
    fun page_names_retained() {
        val state = ControlSurfaceUiState.ActiveProfile(
            profileId = "p1",
            profileName = "Test",
            pages = listOf(PageMessage("pg1", "Main Page", emptyList()))
        )
        val items = ControlSurfacePresentationMapper.map(state)
        val pageHeader = items[1] as ControlSurfacePresentationItem.PageHeader
        assertEquals("Main Page", pageHeader.pageName)
    }

    // C6: page IDs are retained exactly
    @Test
    fun page_ids_retained_exactly() {
        val state = ControlSurfaceUiState.ActiveProfile(
            profileId = "p1",
            profileName = "Test",
            pages = listOf(PageMessage("page/456", "X", emptyList()))
        )
        val items = ControlSurfacePresentationMapper.map(state)
        val pageHeader = items[1] as ControlSurfacePresentationItem.PageHeader
        assertEquals("page/456", pageHeader.pageId)
    }

    // C7: all buttons are emitted
    @Test
    fun all_buttons_are_emitted() {
        val state = ControlSurfaceUiState.ActiveProfile(
            profileId = "p1",
            profileName = "Test",
            pages = listOf(
                PageMessage("pg1", "Main", listOf(
                    ButtonMessage("b1", "A"),
                    ButtonMessage("b2", "B"),
                )),
                PageMessage("pg2", "Git", listOf(
                    ButtonMessage("b3", "C"),
                )),
            )
        )
        val items = ControlSurfacePresentationMapper.map(state)
        val btnTiles = items.filterIsInstance<ControlSurfacePresentationItem.ButtonTile>()
        assertEquals(3, btnTiles.size)
    }

    // C8: button order is preserved within each page
    @Test
    fun button_order_preserved() {
        val state = ControlSurfaceUiState.ActiveProfile(
            profileId = "p1",
            profileName = "Test",
            pages = listOf(PageMessage("pg1", "Main", listOf(
                ButtonMessage("b2", "Second"),
                ButtonMessage("b1", "First"),
            )))
        )
        val items = ControlSurfacePresentationMapper.map(state)
        val btnTiles = items.filterIsInstance<ControlSurfacePresentationItem.ButtonTile>()
        assertEquals("b2", btnTiles[0].buttonId)
        assertEquals("b1", btnTiles[1].buttonId)
    }

    // C9: button labels are retained
    @Test
    fun button_labels_retained() {
        val state = ControlSurfaceUiState.ActiveProfile(
            profileId = "p1",
            profileName = "Test",
            pages = listOf(PageMessage("pg1", "Main", listOf(ButtonMessage("b1", "Compile"))))
        )
        val items = ControlSurfacePresentationMapper.map(state)
        val btnTile = items[2] as ControlSurfacePresentationItem.ButtonTile
        assertEquals("Compile", btnTile.label)
    }

    // C10: button IDs are retained exactly
    @Test
    fun button_ids_retained_exactly() {
        val state = ControlSurfaceUiState.ActiveProfile(
            profileId = "p1",
            profileName = "Test",
            pages = listOf(PageMessage("pg1", "Main", listOf(ButtonMessage("btn_789", "Y"))))
        )
        val items = ControlSurfacePresentationMapper.map(state)
        val btnTile = items[2] as ControlSurfacePresentationItem.ButtonTile
        assertEquals("btn_789", btnTile.buttonId)
    }

    // C11: empty pages list does not crash
    @Test
    fun empty_pages_does_not_crash() {
        val state = ControlSurfaceUiState.ActiveProfile(
            profileId = "p1",
            profileName = "Test",
            pages = emptyList()
        )
        val items = ControlSurfacePresentationMapper.map(state)
        assertEquals(1, items.size)
        assertTrue(items[0] is ControlSurfacePresentationItem.ProfileHeader)
    }

    // C12: a page with empty buttons does not crash
    @Test
    fun empty_buttons_does_not_crash() {
        val state = ControlSurfaceUiState.ActiveProfile(
            profileId = "p1",
            profileName = "Test",
            pages = listOf(PageMessage("pg1", "Main", emptyList()))
        )
        val items = ControlSurfacePresentationMapper.map(state)
        assertEquals(2, items.size)
        assertTrue(items[1] is ControlSurfacePresentationItem.PageHeader)
    }

    // C13: NoProjection maps to NoContent
    @Test
    fun no_projection_maps_to_no_content() {
        val items = ControlSurfacePresentationMapper.map(ControlSurfaceUiState.NoProjection)
        assertEquals(1, items.size)
        val nc = items[0] as ControlSurfacePresentationItem.NoContent
        assertEquals("No projection", nc.label)
    }

    // C14: NoActiveProfile maps to NoContent
    @Test
    fun no_active_profile_maps_to_no_content() {
        val items = ControlSurfacePresentationMapper.map(ControlSurfaceUiState.NoActiveProfile)
        assertEquals(1, items.size)
        val nc = items[0] as ControlSurfacePresentationItem.NoContent
        assertEquals("No active profile", nc.label)
    }

    // C15: complete structure — profile + page + buttons in order
    @Test
    fun complete_structure_ordering() {
        val state = ControlSurfaceUiState.ActiveProfile(
            profileId = "p1",
            profileName = "Coding",
            pages = listOf(
                PageMessage("pg1", "Build", listOf(
                    ButtonMessage("b1", "Compile"),
                    ButtonMessage("b2", "Test"),
                )),
                PageMessage("pg2", "Git", listOf(
                    ButtonMessage("b3", "Status"),
                    ButtonMessage("b4", "Diff"),
                    ButtonMessage("b5", "Push"),
                )),
                PageMessage("pg3", "Tools", listOf(
                    ButtonMessage("b6", "Terminal"),
                )),
            )
        )
        val items = ControlSurfacePresentationMapper.map(state)
        assertEquals(1 + 3 + 6, items.size)
        assertEquals("Coding", (items[0] as ControlSurfacePresentationItem.ProfileHeader).profileName)
        assertEquals("Build", (items[1] as ControlSurfacePresentationItem.PageHeader).pageName)
        assertEquals("Compile", (items[2] as ControlSurfacePresentationItem.ButtonTile).label)
        assertEquals("Test", (items[3] as ControlSurfacePresentationItem.ButtonTile).label)
        assertEquals("Git", (items[4] as ControlSurfacePresentationItem.PageHeader).pageName)
        assertEquals("Status", (items[5] as ControlSurfacePresentationItem.ButtonTile).label)
        assertEquals("Diff", (items[6] as ControlSurfacePresentationItem.ButtonTile).label)
        assertEquals("Push", (items[7] as ControlSurfacePresentationItem.ButtonTile).label)
        assertEquals("Tools", (items[8] as ControlSurfacePresentationItem.PageHeader).pageName)
        assertEquals("Terminal", (items[9] as ControlSurfacePresentationItem.ButtonTile).label)
    }
}
