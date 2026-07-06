# Discovery

> How mobile and desktop devices find each other on the local network.

## Requirements

- No cloud dependency — discovery must work entirely on the local network.
- Zero configuration — no IP addresses, no hostnames, no manual setup.
- Multiple desktops on the same network must be distinguishable.
- Devices come and go dynamically (desktop starts/stops, mobile moves between networks).

## Connection Recovery Fallback Chain

When an established connection is lost, the mobile attempts recovery in the following order:

```
1. Reconnect using existing session
        ↓ (fails)
2. Re-discover desktop via mDNS/SSDP
        ↓ (fails)
3. Resolve hostname (DNS / NetBIOS)
        ↓ (fails)
4. Manual IP entry (emergency fallback)
```

Each step must succeed or fail cleanly with user-visible feedback before advancing to the next.

### Test Scenarios

- Desktop hostname changes.
- DHCP lease renewal (desktop gets new IP).
- Router replacement (new subnet, all cached addresses invalid).
- Multiple network adapters (Ethernet + Wi-Fi on same desktop).

## Discovery Mechanism

TBD — evaluated during v0.1. Candidates:

| Method | Pros | Cons |
|--------|------|------|
| mDNS (Bonjour/Avahi) | Standard, zero-conf, cross-platform | Requires mDNS stack on each platform |
| SSDP (UPnP) | Windows-native, widely supported | Verbose, less common on mobile |
| Manual IP entry | Simple emergency fallback | Poor UX, error-prone — last resort only |

## Flow

1. Desktop agent starts and begins broadcasting its presence on the LAN.
2. Mobile app scans the network for desktop announcements.
3. Mobile displays a list of discovered desktops (name, OS, protocol version).
4. User selects a desktop to initiate pairing.

## Announced Information

Each discovery announcement includes:

- `deviceName` — Human-readable name (hostname or user-configured)
- `deviceId` — Persistent unique identifier
- `deviceType` — `desktop`
- `protocolVersions` — List of supported protocol versions
- `os` — OS type and version
- `capabilities` — Summary of available plugin capabilities (counts, not details)

## Addressing

- Android apps on the same device cannot discover each other via network (security model).
- A future local-loopback path (Unix socket or ADB forward) may be added for mobile-desktop on the same machine.
