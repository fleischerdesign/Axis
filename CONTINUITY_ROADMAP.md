# Continuity System: Roadmap & Improvements

This document outlines the current state of the Axis Continuity system and proposes architectural, logical, and UI/UX improvements to move it toward a production-ready state.

## Current State & Strengths
- **Reactive Architecture:** Fully integrated into the Axis `Service` and `Store` pattern.
- **Async Execution:** Heavy lifting (networking, input capture) runs in background threads without blocking the UI.
- **Layer Shell Integration:** Uses invisible 2px windows for edge detection, providing a native desktop feel.
- **Bidirectional Sharing:** Supports both being a "Sharer" and a "Receiver" with a robust switch protocol.
- **Protocol:** Custom length-prefixed JSON protocol over TCP with versioning and heartbeat.

---

## 1. Architectural Improvements (Backend)

### 1.1 Module Refactoring
The `src/services/continuity/mod.rs` file has grown to over 1000 lines.
- **Action:** Split `handle_connection_event` and its large message matching block into a dedicated `message_handler.rs` or `protocol_handler.rs`.
- **Goal:** Improve maintainability and reduce cognitive load when navigating the service.

### 1.2 Configuration Persistence
Peer arrangements (e.g., "Device B is to the Right of Device A") are currently RAM-only.
- **Action:** Implement a persistence layer using `serde_json` to save `peer_configs` to `~/.config/axis/continuity_peers.json`.
- **Goal:** User settings should survive a shell restart.

### 1.3 Dynamic Resolution Handling
Screen dimensions are queried once at startup via Niri IPC.
- **Action:** Listen for Niri output events or GTK monitor signals to update `screen_width` and `screen_height` dynamically.
- **Goal:** Support hot-plugging monitors and resolution changes without breaking edge detection.

---

## 2. Logic & Security Enhancements

### 2.1 Edge Detection Refinement
The 2px edge window is functional but can be triggered accidentally or skipped by very high-speed cursor movements.
- **Action:**
    - Implement a "dwell time" (require the cursor to stay on the edge for >50ms).
    - Add a velocity check (don't trigger if the cursor is just passing through quickly).
- **Goal:** Reduce "ghost transitions" during normal mouse usage.

### 2.2 Feature Toggles
Clipboard synchronization is currently mandatory when connected.
- **Action:** Add a `clipboard_enabled` flag to `PeerConfig`.
- **Goal:** Allow users to share only input without exposing their clipboard.

### 2.3 Secure Randomization
PIN generation currently uses a simple modulo on a UUID.
- **Action:** Use a proper RNG (like `rand` crate) for PIN generation.
- **Goal:** Improved cryptographic randomness for the pairing process.

---

## 3. UI/UX Improvements (Frontend)

### 3.1 Offset Fine-Tuning
The protocol supports `offset` for `PeerArrangement` (essential for mismatched monitor sizes), but the UI lacks a way to set it.
- **Action:** Add a slider or spin button in `ContinuityPage` to adjust the vertical/horizontal offset.
- **Goal:** Perfect alignment of cursor transitions between different screens.

### 3.2 Visual Feedback
The red debug line in `continuity_capture.rs` is useful for development but intrusive for users.
- **Action:** 
    - Hide the line by default.
    - Provide a "Show Edges" debug toggle in the settings.
    - Add a subtle animation (e.g., a brief glow) on the target edge when a connection is established.
- **Goal:** Polish the visual experience while keeping debug tools accessible.

### 3.3 Localization & Consistency
The UI currently uses German strings while the rest of the project is in English.
- **Action:** Standardize on English UI strings to match `AGENTS.md` standards, or implement a basic `i18n` (internationalization) system.
- **Goal:** Maintain project-wide consistency.

---

## 4. Advanced Features & Production Readiness

### 4.1 Encryption (TLS / Noise Protocol)
Currently, input data (including key presses) is sent as plaintext JSON over the local network.
- **Action:** Implement a secure handshake (e.g., using `rustls` or `Noise Protocol`) after the initial PIN confirmation.
- **Goal:** Protect sensitive data like passwords from being sniffed on the local network.

### 4.2 Auto-Connect for Trusted Peers
Users currently have to manually click "Connect" every time.
- **Action:** Implement a "Trusted" flag for peers that have successfully completed a PIN handshake. Automatically attempt a connection when a trusted peer is discovered via mDNS.
- **Goal:** Seamless "magic" connectivity when devices are near each other.

### 4.3 Multi-Monitor Support (Local)
The current implementation queries only the first output from Niri.
- **Action:** Detect all physical outputs and create edge windows on the correct monitor based on where the user's arrangement places the peer.
- **Goal:** Full support for workstations with multiple displays.

### 4.4 Protocol Optimization (Binary Input)
JSON parsing for high-frequency events (like 1000Hz mouse movement) introduces unnecessary CPU overhead and latency.
- **Action:** Use a compact binary format (e.g., `bincode` or a custom bit-packed layout) for `CursorMove` and `KeyPress` messages.
- **Goal:** Sub-millisecond latency for a "local-feeling" cursor on the remote machine.

### 4.5 Safety: Input Capture Watchdog
If the Axis process crashes while capturing input, the local mouse might remain "trapped."
- **Action:** Implement a watchdog or kernel-level fallback that automatically releases `evdev` devices if the heartbeat from the main Axis thread stops for more than 5 seconds.
- **Goal:** Ensure the user never loses control of their local machine due to a software crash.

---

## 5. Phase 3: Ecosystem Integration (Future)

### 5.1 Drag & Drop File Transfer
- **Action:** Implement a file-streaming service over the peer connection. Trigger a transfer when a "Drop" event occurs on the screen edge.
- **Goal:** Move files between computers as easily as moving the cursor.

### 5.2 Notification Synchronization
- **Action:** Relay system notifications between connected peers via the `NotificationsService`.
- **Goal:** See laptop alerts on your desktop monitor while working.

### 5.3 Shared Media Control (MPRIS)
- **Action:** Sync play/pause/skip status between devices.
- **Goal:** Control music playing on your desktop from your laptop's Axis bar.

### 5.4 Remote App Launching (Handoff)
- **Action:** Send URLs or App IDs to a peer to open them instantly.
- **Goal:** "Continue reading this article on my other device" with one click.

### 5.5 Multi-Device Layout (Grid Support)
- **Action:** Expand `PeerArrangement` to support more than two devices in a 2D grid (e.g., tablet below the main monitor).
- **Goal:** Support complex multi-machine workstations.

### 5.6 Remote Battery & Status Monitoring
- **Action:** Exchange system metadata (battery level, CPU load) in the heartbeat message.
- **Goal:** Monitor your laptop's health directly from your desktop's status bar.

---

## 6. Phase 4: Advanced Hardware & Workflow (Experimental)

### 6.1 Biometric Unlock Relay
- **Action:** Use the fingerprint sensor of a laptop to unlock a connected desktop via a secure challenge-response.
- **Goal:** Unified authentication across all your devices.

### 6.2 Virtual Camera/Mic Sharing
- **Action:** Stream camera/microphone data as a virtual PipeWire source to the peer.
- **Goal:** Use your laptop's webcam as a high-quality camera for your desktop.

### 6.3 Window "Teleportation" (Window Handoff)
- **Action:** Use Niri IPC to "push" a window state to a peer, allowing the app to resume there.
- **Goal:** Move entire workspaces between physical machines.

### 6.4 Proximity-based Lock
- **Action:** Monitor signal strength (RSSI) of the peer connection. Lock the screen when the peer moves out of range.
- **Goal:** Automated security for mobile/desktop setups.

### 6.5 Shared Terminal / Pair Programming Mode
- **Action:** Implement focused input forwarding only for a specific window/terminal.
- **Goal:** Allow two users to work on the same machine with separate cursors.

### 6.6 Network Tethering Quick-Toggle
- **Action:** Easily share a desktop's Ethernet connection with a laptop over the peer link.
- **Goal:** Instant high-speed internet sharing for docked devices.

---

## 7. Phase 5: Seamless Workflow & Context

### 7.1 Shared Clipboard History
- **Action:** Sync a buffer of the last 10 clipboard items across devices.
- **Goal:** Access older copied snippets regardless of which device you are currently using.

### 7.2 Unified Search & Global Launcher
- **Action:** Allow the Axis launcher to query search results from connected peers.
- **Goal:** Find and open files or apps on your laptop directly from your desktop.

### 7.3 Contextual Resume (Activity Handoff)
- **Action:** Track active files/projects and offer a "Resume" button when switching devices.
- **Goal:** Seamlessly jump between workstations without losing your place in a document.

### 7.4 Global Media Sink (Audio Mirroring)
- **Action:** Route audio from one device to the speakers/headphones of another via PipeWire.
- **Goal:** Listen to your laptop's audio through your desktop's high-quality speakers.

---

## 8. Phase 6: Hardware & System Virtualization (Experimental)

### 8.1 Virtual Display Extension (Sidecar)
- **Action:** Use a laptop or tablet as a wireless second monitor for a desktop.
- **Goal:** Expand your screen real estate using existing hardware over the network.

### 8.2 Remote Peripheral Sharing
- **Action:** Mount USB drives or other peripherals connected to a peer as if they were local.
- **Goal:** Unified hardware access across all networked machines.

### 8.3 Shared Touchpad & Gesture Relay
- **Action:** Forward multi-touch gestures (e.g., 3-finger swipes) from a laptop touchpad to a desktop.
- **Goal:** Use laptop-specific input features to control your desktop compositor.

### 8.4 Distributed Resource Monitoring
- **Action:** Create a dashboard showing CPU, RAM, and Temperature for all connected peers.
- **Goal:** Keep an eye on your "home lab" or multi-machine setup from a single bar.

### 8.5 Remote Hardware Control
- **Action:** Control a peer's screen brightness or volume using local media keys.
- **Goal:** Unified control of all physical hardware in your immediate vicinity.
