# Spike ep-005: Network Resilience

> Validate behavior under hostile or unstable network conditions.

## Question

How does Auto-Mat-Deck behave when the network is unreliable, reconfigured, or enterprise-managed?

## Scope

- DHCP lease renewal and IP address changes.
- Hostname resolution (mDNS vs. DNS vs. NetBIOS).
- AP isolation (guest networks, public Wi-Fi).
- Captive portals (hotels, airports, cafes).
- Enterprise Wi-Fi (802.1X, certificate-based auth).
- Subnet changes (router replacement, VPN activation).
- Multiple network adapters (Ethernet + Wi-Fi, virtual adapters).
- Packet loss, jitter, and high latency.
- IPv4 vs. IPv6: discovery and connectivity on both stacks.

## Out of Scope

- Pairing security (covered by EP-001).
- Application-level protocol (covered by EP-001).
- Device lifecycle (covered by EP-002, EP-003).

## Status

Planned. Not started until EP-001 completes.
