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

data class StepResult(
    val stepIndex: Int,
    val actionId: String,
    val executed: Boolean,
    val error: String?,
)

data class ControlInvokeResult(
    val buttonId: String,
    val accepted: Boolean,
    val reason: String?,
    val executed: Boolean?,
    val executionError: String?,
    val steps: List<StepResult>,
)

data class TriggerInvokeResult(
    val triggerId: String,
    val accepted: Boolean,
    val reason: String?,
    val executed: Boolean?,
    val executionError: String?,
)

class SpikeMessageDispatcher {
    var uiState: ControlSurfaceUiState = ControlSurfaceUiState.NoProjection
        private set

    var lastRaw: ControlSurfaceStateMessage? = null
        private set

    var lastInvokeResult: ControlInvokeResult? = null
        private set

    var triggers: List<TriggerMessage> = emptyList()
        private set

    var lastTriggerResult: TriggerInvokeResult? = null
        private set

    var triggerHistory: List<TriggerHistoryRecord> = emptyList()
        private set

    fun handle(text: String) {
        val json = try { JSONObject(text) } catch (_: Exception) { return }
        val msgType = json.optString("type")
        when (msgType) {
            "active_profile_state" -> handleActiveProfileState(json)
            "control_surface_state" -> handleControlSurfaceState(json)
            "control_invoke_result" -> handleControlInvokeResult(json)
            "trigger_state" -> handleTriggerState(json)
            "trigger_invoke_result" -> handleTriggerInvokeResult(json)
            "trigger_history" -> handleTriggerHistory(json)
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
        lastInvokeResult = null
        triggers = emptyList()
        lastTriggerResult = null
        triggerHistory = emptyList()
    }

    private fun handleControlInvokeResult(json: JSONObject) {
        val steps = if (json.has("steps")) {
            val arr = json.getJSONArray("steps")
            (0 until arr.length()).map { i ->
                val s = arr.getJSONObject(i)
                StepResult(
                    stepIndex = s.optInt("step_index", i),
                    actionId = s.optString("action_id", ""),
                    executed = s.optBoolean("executed", false),
                    error = if (s.has("error")) s.getString("error") else null,
                )
            }
        } else {
            emptyList()
        }

        lastInvokeResult = ControlInvokeResult(
            buttonId = json.optString("button_id", ""),
            accepted = json.optBoolean("accepted", false),
            reason = if (json.has("reason")) json.getString("reason") else null,
            executed = if (json.has("executed")) json.getBoolean("executed") else null,
            executionError = if (json.has("execution_error")) json.getString("execution_error") else null,
            steps = steps,
        )
    }

    private fun handleTriggerState(json: JSONObject) {
        val tsm = TriggerStateMessage.fromJson(json) ?: return
        triggers = tsm.triggers
    }

    private fun handleTriggerInvokeResult(json: JSONObject) {
        val triggerId = json.optString("trigger_id", "")
        if (triggerId.isEmpty()) return
        lastTriggerResult = TriggerInvokeResult(
            triggerId = triggerId,
            accepted = json.optBoolean("accepted", false),
            reason = if (json.has("reason")) json.getString("reason") else null,
            executed = if (json.has("executed")) json.getBoolean("executed") else null,
            executionError = if (json.has("execution_error")) json.getString("execution_error") else null,
        )
    }

    private fun handleTriggerHistory(json: JSONObject) {
        val msg = TriggerHistoryMessage.fromJson(json) ?: return
        triggerHistory = msg.records
    }
}
