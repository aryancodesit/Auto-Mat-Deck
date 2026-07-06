# Spike ep-002: Android Background Execution & Doze

> Validate Android's ability to maintain connectivity and execute automations while the app is in the background.

## Question

Can Auto-Mat-Deck's Android app maintain a WebSocket connection and respond to triggers while the device is Dozing, battery-optimized, or backgrounded?

## Scope

- Foreground Service lifecycle and notification requirements.
- Doze mode behavior (light vs. deep Doze).
- Battery optimization exemptions (request vs. required).
- Network access during standby.
- Wake lock usage and timeout behavior.
- Connection re-establishment after app kill.
- Android API level minimum determination (target API 26+).

## Out of Scope

- Pairing or transport security (covered by EP-001).
- Discovery (covered by EP-001).
- Protocol message framing.

## Status

Planned. Not started until EP-001 completes.
