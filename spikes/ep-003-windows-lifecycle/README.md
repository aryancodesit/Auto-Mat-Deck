# Spike ep-003: Windows Service, Firewall & Sleep/Wake

> Validate Windows-specific lifecycle requirements for the desktop agent.

## Question

Can the desktop agent run reliably as a background service on Windows 10/11 through sleep, wake, firewall, and startup scenarios?

## Scope

- Windows Service vs. system tray process: install, start, stop, restart.
- Windows Firewall: first-launch prompt, binding to local subnet only, admin vs. user-level.
- Startup behavior: auto-start on boot, delayed start, user login vs. service start.
- Sleep / resume: connection state recovery, timing measurements.
- Power events: handling suspend, hibernate, and shutdown notifications.
- Multiple user sessions: fast user switching, remote desktop.
- Windows Defender SmartScreen / antivirus false positives.

## Out of Scope

- Linux or macOS lifecycle (future spikes).
- Discovery or pairing (covered by EP-001).
- Protocol or message format.

## Status

Planned. Not started until EP-001 completes.
