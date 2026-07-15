package com.automatdeck.spike

sealed interface ControlSurfacePresentationItem {
    data class ProfileHeader(
        val profileId: String,
        val profileName: String,
    ) : ControlSurfacePresentationItem

    data class PageHeader(
        val pageId: String,
        val pageName: String,
    ) : ControlSurfacePresentationItem

    data class ButtonTile(
        val buttonId: String,
        val label: String,
    ) : ControlSurfacePresentationItem

    data class NoContent(val label: String) : ControlSurfacePresentationItem
}

object ControlSurfacePresentationMapper {
    fun map(state: ControlSurfaceUiState): List<ControlSurfacePresentationItem> {
        return when (state) {
            is ControlSurfaceUiState.NoProjection -> listOf(
                ControlSurfacePresentationItem.NoContent("No projection")
            )
            is ControlSurfaceUiState.NoActiveProfile -> listOf(
                ControlSurfacePresentationItem.NoContent("No active profile")
            )
            is ControlSurfaceUiState.ActiveProfile -> {
                val items = mutableListOf<ControlSurfacePresentationItem>()
                items.add(ControlSurfacePresentationItem.ProfileHeader(
                    profileId = state.profileId,
                    profileName = state.profileName,
                ))
                for (page in state.pages) {
                    items.add(ControlSurfacePresentationItem.PageHeader(
                        pageId = page.pageId,
                        pageName = page.name,
                    ))
                    for (btn in page.buttons) {
                        items.add(ControlSurfacePresentationItem.ButtonTile(
                            buttonId = btn.buttonId,
                            label = btn.label,
                        ))
                    }
                }
                items
            }
        }
    }
}
