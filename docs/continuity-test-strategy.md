# Continuity Test Strategy & Architecture Fix

## Root Cause Analysis

The central problem is an architecture violation inside the Infrastructure layer.
`ContinuityInner::run()` (`inner/mod.rs:211`) instantiates all dependencies as concrete types:

```rust
let mut discovery = AvahiDiscovery::new();
let mut connection = TcpConnectionProvider::new();
let mut clipboard = WaylandClipboard::new();
let mut injection = WaylandInjection::new();
let mut capture = EvdevCapture::new();
```

This makes the entire system untestable without two physical devices. The rest of the
codebase follows hexagonal architecture (`ContinuityProvider` trait -> `ContinuityService`
adapter), but *inside* the infrastructure layer the principle was broken.

### Consequences

1. No testability — netzwork, Wayland, PipeWire, Avahi are all real
2. Core logic (90% of the code) is locked inside I/O-coupled event loop
3. Every change requires two-device manual testing
4. Bugs can only be found by running on real hardware

---

## Phase 1: Domain Ports (Dependency Inversion)

**File:** `crates/axis-domain/src/ports/continuity.rs`

Five new traits abstracting infrastructure concerns, following the existing hexagonal pattern:

| Port | Abstracts | Existing Implementation |
|------|-----------|------------------------|
| `ContinuityNetworkPort` | TCP send/recv, connect, listen, disconnect | `TcpConnectionProvider` |
| `ContinuityAudioPort` | Audio capture/playback, device enumeration | `AudioStreamManager` + `pipewire_devices` |
| `ContinuityInputPort` | evdev capture + Wayland injection | `EvdevCapture` + `WaylandInjection` |
| `ContinuityClipboardPort` | Clipboard monitoring + setting | `WaylandClipboard` |
| `ContinuityDiscoveryPort` | mDNS register/browse/stop | `AvahiDiscovery` |

Zero-cost in production: `CmdContext` stays concretely typed; trait usage only for
`#[cfg(test)]` dyn dispatch.

### Status
- [ ] `ContinuityNetworkPort` — connect, listen, send_message, disconnect_active, active_write_tx
- [ ] `ContinuityAudioPort` — start_capture, stop_capture, play_chunk, stop_playback, list_devices
- [ ] `ContinuityInputPort` — start_capture, stop_capture, start_injection, stop_injection, warp, inject
- [ ] `ContinuityClipboardPort` — start_monitoring, stop_monitoring, set_content, get_content
- [ ] `ContinuityDiscoveryPort` — register, browse, stop

---

## Phase 2: Refactor CmdContext

From concrete types to trait references:

```rust
pub(crate) struct CmdContext<'a> {
    pub network: &'a mut dyn ContinuityNetworkPort,
    pub audio: &'a dyn ContinuityAudioPort,
    pub input: &'a mut dyn ContinuityInputPort,
    pub clipboard: &'a mut dyn ContinuityClipboardPort,
    pub discovery: &'a mut dyn ContinuityDiscoveryPort,
    // channels unchanged
}
```

### Status
- [ ] CmdContext uses traits
- [ ] All `handle_*` methods updated to use trait methods
- [ ] Production adapters implement all traits
- [ ] `cargo clippy -- -D warnings && cargo test` passes

---

## Phase 3: Test Adapters

**File:** `crates/axis-infrastructure/src/adapters/continuity/testing.rs`
(`#[cfg(test)]` only)

| Mock | Behavior |
|------|----------|
| `LoopbackNetwork` | Two `mpsc` channels simulate a TCP connection between two `ContinuityInner` instances in the same process |
| `MockAudio` | Records sent PCM chunks (for assertions), provides configurable chunks for playback |
| `MockInput` | Records `InternalInputEvent`s, simulates cursor warp |
| `MockClipboard` | In-memory string (read/write) |
| `MockDiscovery` | Static peer list, no real Avahi |

### Status
- [ ] `LoopbackNetwork`: paired mpsc channels, ConnectionEvent forwarding, Message routing
- [ ] `MockAudio`: Vec<Vec<u8>> capture log, Vec<Vec<u8>> playback queue, list_devices stub
- [ ] `MockInput`: event recording, warp coordinate logging
- [ ] `MockClipboard`: String storage, monitoring simulation
- [ ] `MockDiscovery`: pre-configured PeerInfo list

---

## Phase 4: Integration Test Harness

**File:** `crates/axis-infrastructure/src/adapters/continuity/inner/tests.rs`

```rust
#[cfg(test)]
mod integration {
    struct ContinuityTestHarness {
        local: ContinuityInner,
        remote: ContinuityInner,
        // shared mock adapters
    }

    impl ContinuityTestHarness {
        async fn connect_trusted(&mut self);
        async fn sync_config_and_expect(&mut self, config: PeerConfig);
        async fn send_audio_and_assert(&mut self, pcm: Vec<u8>);
        async fn toggle_setting_and_assert(&mut self, setting: &str, value: bool);
    }
}
```

### Test Matrix (RED — should fail on current code)

| ID | Scenario | Category | Status |
|----|----------|----------|--------|
| T1 | Handshake trusted (both directions) | Connection | [ ] |
| T2 | Handshake untrusted (PIN flow) | Connection | [ ] |
| T3 | ConfigSync: arrangement mirroring | Sync | [ ] |
| T4 | ConfigSync: audio_direction mirroring | Sync | [ ] |
| T5 | ConfigSync: version conflict (both v0) | Sync | [ ] |
| T6 | AudioChunk: capture → network → playback | Audio | [ ] |
| T7 | ClipboardUpdate: set → network → get | Clipboard | [ ] |
| T8 | Disconnect + Reconnect | Connection | [ ] |
| T9 | Peer-ID-Matching: UUID vs hostname | Sync | [ ] |
| T10 | active_peer_config() fallback edge-cases | Sync | [ ] |
| T11 | persist_known_peers() audio_direction roundtrip | Persistence | [ ] |

---

## Phase 5: Bug Fixes (RED → GREEN)

Each bug gets a test that FAILS on current code, then the fix makes it GREEN.

| # | Bug | Test | Fix | Status |
|---|-----|------|-----|--------|
| B1 | PIN exchange broken — both sides generate independent random PINs, `handle_pin_request()` overwrites with peer's PIN, `handle_pin_confirm()` check always fails | `test_pin_exchange_untrusted()` | Only initiator generates PIN, sends to receiver. Receiver displays initiator's PIN. Both confirm same PIN. | [ ] |
| B2 | `is_peer_active` uses fragile multi-strategy string matching (starts_with, equality, known_peers fallback) causing ConfigSync to never be sent | `test_peer_id_matching_with_uuid()` | UUID-only comparison. Remove starts_with and fallback chains. | [ ] |
| B3 | `audio_direction` not persisted in `known_peers.json` — resets to `Off` on restart | `test_audio_direction_persistence_roundtrip()` | Add `audio_direction: Option<AudioStreamDirection>` to `KnownPeer`. Persist in `to_peer_config()` and `persist_known_peers()`. | [ ] |
| B4 | ConfigSync version `>=` causes thrashing when both peers have v0 | `test_configsync_version_thrashing()` | Use `>` instead of `>=`. Keep `is_initial_adopt` for bootstrap case. | [ ] |
| B5 | `PeerDetailPage.update_status()` falls back to `peer_configs.values().next()` — displays wrong peer's config | `test_peer_detail_resolves_correct_config()` | Exact UUID match or `None`. No random fallback. | [ ] |
| B6 | `ContinuityCipher` is defined but never instantiated — all data travels in plaintext over TCP | `test_cipher_instantiated_on_handshake()` | Instantiate cipher in `handle_hello()`, encrypt AudioChunk/ClipboardUpdate/DragChunk messages. | [ ] |

---

## Phase 6: Audio Pipeline (pipewire-rs)

Replace `pw-record`/`pw-cat` CLI child processes with `pipewire` Rust bindings.

**From:**
```
pw-record --target @DEFAULT_MONITOR@ --raw --format=s16 --rate=44100 --channels=2 --latency=20ms -
pw-cat --playback --raw --format=s16 --rate=44100 --channels=2 --latency=20ms -
```

**To:**
```rust
use pipewire::stream::PipeWireStream;
// Proper stream management, format negotiation, buffer control
```

### Benefits
- No orphaned child processes on crash
- Proper error handling (no `stderr: null()`)
- Jitter buffer and sequence numbers for audio packets
- Direct device enumeration without `pw-dump` parsing

### Status
- [ ] `PipeWireCaptureStream` implements `ContinuityAudioPort` (capture side)
- [ ] `PipeWirePlaybackStream` implements `ContinuityAudioPort` (playback side)
- [ ] Device enumeration via pipewire-rs API
- [ ] Integration test verifies audio pipeline end-to-end

---

## Phase 7: Structured Logging (tracing)

Replace `log` with `tracing` for the continuity subsystem.

```rust
// Before: context-free
info!("[continuity] adopting config from peer (v{})...", version);

// After: spans with correlation IDs
let span = tracing::info_span!("config_sync", peer_id = %peer_id, version = version);
let _guard = span.enter();
tracing::info!(arrangement = ?args.arrangement, "adopting config");
tracing::debug!(?config, "full config state after merge");
```

### Log Levels
| Level | Content |
|-------|---------|
| `ERROR` | Connection failures, protocol violations |
| `WARN` | Timeouts, rejected operations |
| `INFO` | State transitions, connections, config changes |
| `DEBUG` | Full message payloads (ConfigSync, ClipboardUpdate, etc.) |
| `TRACE` | Raw PCM chunk sizes, heartbeat ticks, internal events |

### Status
- [ ] Add `tracing` dependency to `axis-infrastructure`
- [ ] Create named spans for each `handle_*` method
- [ ] Add correlation IDs to track messages across peers
- [ ] `RUST_LOG=axis_infrastructure::continuity=trace` dumps full message contents

---

## Execution Order

1. Phase 1: Domain ports defined (1 file, ~100 lines, no existing code changes)
2. Phase 2: CmdContext refactored + adapters implement new traits
3. Phase 3: Test adapters built (1 file, #\[cfg\(test\)\], no existing code changes)
4. Phase 4: RED tests written (demonstrate bugs exist)
5. Phase 5: Bugs fixed → GREEN tests
6. Phase 6: Audio pipeline migrated to pipewire-rs
7. Phase 7: tracing instrumentation

Each phase is self-contained and verifiable via `cargo test -p axis-infrastructure`.
