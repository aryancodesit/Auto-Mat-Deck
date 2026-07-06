# ADR-006: Transport Evaluation

**Status:** Proposed
**Date:** 2026-07-06

## Context

The communication protocol document currently lists several possible transports (WebSocket, TCP, USB/ADB, QUIC) and formats (Protocol Buffers, JSON, FlatBuffers, CBOR) as open options. This ambiguity blocks implementation — mobile and desktop cannot begin coding until the transport is chosen.

Rather than making an architectural guess, we will validate candidates with a real prototype (see `spikes/ep-001-discovery-pairing/`).

## Decision

Defer the transport selection until after spike ep-001 completes.

### Evaluation Criteria

| Criterion | Weight | Notes |
|-----------|--------|-------|
| Android support | Required | Must work on API 26+ without exotic dependencies |
| Windows support | Required | Must work on stock Windows 10/11 without admin |
| Bidirectional | Required | Server and client roles needed on both sides |
| Encryption | Required | Must support or integrate with Noise/TLS |
| NAT/firewall resilience | High | Must work on consumer home networks |
| Latency | Medium | < 100 ms round-trip for command/response |
| Throughput | Low | Protocol messages are small (< 1 KB typical) |
| Debugging | Medium | Should be inspectable with common tools |

### Candidates

| Transport | Android | Windows | Encryption | Bidirectional | Assessment |
|-----------|---------|---------|------------|---------------|------------|
| WebSocket | ✅ Native | ✅ Native | ✅ WSS or Noise over raw | ✅ Full-duplex | Strong candidate |
| Raw TCP | ✅ Okio/Java NIO | ✅ .NET TcpClient | ⚠️ Must add Noise/TLS yourself | ✅ | Viable, more work |
| QUIC | ⚠️ Cronet dependency | ⚠️ MsQuic / experimental | ✅ Built-in | ✅ | Too immature for v0.1 |
| USB/ADB | ✅ ADB forward | ⚠️ ADB required | ❌ No built-in encryption | ⚠️ Request-response only | Fallback only |
| gRPC | ✅ Via grpc-java | ✅ Via grpc-dotnet | ✅ TLS | ⚠️ Half-duplex streams | Heavy dependency |

### Message Format Candidates

| Format | Binary? | Schema? | Android | Windows | Assessment |
|--------|---------|---------|---------|---------|------------|
| JSON | No | Optional | ✅ Native | ✅ Native | Great for prototyping |
| Protocol Buffers | Yes | Required | ✅ protobuf-java | ✅ protobuf-net | Strong for production |
| FlatBuffers | Yes | Required | ✅ Native support | ✅ Via external | Zero-copy, more complex |
| CBOR | Yes | Optional | ✅ cbor-java | ✅ cbor-.net | Compact JSON alternative |
| MessagePack | Yes | Optional | ✅ msgpack-java | ✅ msgpack-cli | Fast, small |

## Rationale

- Transport and format are the most coupled decisions in the architecture — changing either after implementation starts is expensive.
- A real prototype on consumer Wi-Fi will surface issues (firewall, NAT, latency, packet loss) that no document can predict.
- The spike is bounded (days, not weeks). The cost of prototyping is far lower than the cost of choosing wrong.

## Consequences

- Desktop and mobile implementation cannot begin until transport is chosen.
- Spike ep-001 must be prioritized before v0.2 (desktop skeleton).
- Transport decision will be recorded in a follow-up ADR after spike results are analyzed.
- The protocol document will be updated once the transport is finalized.
