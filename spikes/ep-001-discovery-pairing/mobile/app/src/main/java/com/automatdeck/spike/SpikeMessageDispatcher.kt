package com.automatdeck.spike

import org.json.JSONObject

sealed interface ControlSurfaceUiState {
    data object NoProjection : ControlSurfaceUiState
    data object NoActiveProfile : ControlSurfaceUiState
    data class ActiveProfile(
        val profileId: String,
        val profileName: String,
        val pages: List<PageMessage>,
    ) : ControlSurfaceUiState
}

class SpikeMessageDispatcher {
    var uiState: ControlSurfaceUiState = ControlSurfaceUiState.NoProjection
        private set

    var lastRaw: ControlSurfaceStateMessage? = null
        private set

    fun handle(text: String) {
        val json = try { JSONObject(text) } catch (_: Exception) { return }
        val msgType = json.optString("type")
        when (msgType) {
            "active_profile_state" -> handleActiveProfileState(json)
            "control_surface_state" -> handleControlSurfaceState(json)
        }
    }

    private fun handleActiveProfileState(json: JSONObject) {
        val aps = ActiveProfileStateMessage.fromJson(json) ?: return
        if (aps.activeProfileId == null) {
            uiState = ControlSurfaceUiState.NoProjection
            lastRaw = null
        }
    }

    private fun handleControlSurfaceState(json: JSONObject) {
        val css = ControlSurfaceStateMessage.fromJson(json) ?: return

        if (css.profileId == null && css.profileName == null && css.pages == null) {
            uiState = ControlSurfaceUiState.NoActiveProfile
            lastRaw = css
            return
        }

        val pid = css.profileId
        val pn = css.profileName
        val pp = css.pages
        if (pid != null && pn != null && pp != null) {
            uiState = ControlSurfaceUiState.ActiveProfile(
                profileId = pid,
                profileName = pn,
                pages = pp,
            )
            lastRaw = css
            return
        }
    }

    fun reset() {
        uiState = ControlSurfaceUiState.NoProjection
        lastRaw = null
    }
}
