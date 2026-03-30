# Axis Settings Application: Design & Strategy

This document outlines the architecture, features, and UI/UX design for a standalone settings application for the Axis Shell.

## 1. Vision & Core Principles

- **Unified Control:** One place to configure everything—from the bar's appearance to complex Continuity arrangements.
- **Native Look & Feel:** Built with **GTK4** and **Libadwaita** to match the shell's visual language.
- **Reactive Updates:** Changes in the settings app should reflect instantly in the shell without requiring a restart.
- **Modular Structure:** Each settings page (Appearance, Networking, Continuity, etc.) should be a separate module, following the shell's own architecture.

---

## 2. Technical Architecture

### 2.1 Communication Strategy
The settings app should be a separate process from the main shell for stability and memory efficiency.
- **IPC Protocol:** Expand the existing **D-Bus interface (`org.axis.Shell`)**.
- **State Sync:** The shell should publish its current `ServiceData` via D-Bus properties (Signals), and the settings app should send `ServiceCmd` equivalents via D-Bus methods.

### 2.2 Configuration Management
- **Persistence:** Use a shared configuration file (e.g., `~/.config/axis/config.json`).
- **Watching:** Use a file watcher (like `notify`) in the shell to reload settings if the file is modified by the settings app.

---

## 3. UI Structure (Pages)

### 3.1 General / Bar
- **Bar Position:** Top/Bottom (and eventually Left/Right).
- **Bar Behavior:** Autohide, Exclusive Zone (Layer Shell), and Layer selection.
- **Islands:** Toggle visibility of Launcher, Clock, Status, and Workspace islands.
- **Interactivity:** Adjust hover delays and animation speeds.

### 3.2 Appearance
- **Theme Selection:** Light/Dark/System (following Adwaita's standard).
- **CSS Editor:** A built-in code editor for `style.css` with a live preview button.
- **Transparency:** Global opacity slider for popups and the bar.
- **Corner Radius:** Adjust the "roundness" of islands and popups.

### 3.3 Continuity (The "Hub")
- **Device Management:** List of trusted/known peers.
- **Visual Arrangement:** A drag-and-drop 2D grid to position peers relative to each other.
- **Offset Tuning:** Fine-tuning sliders for pixel-perfect edge transitions.
- **Feature Toggles:** Enable/Disable Clipboard, Audio, or Drag & Drop per device.
- **Encryption Status:** View security levels and regenerate PINs/Keys.

### 3.4 Services & System
- **Service Toggle:** Manually enable/disable background services (Bluetooth, KDE Connect, Network, etc.).
- **Log Viewer:** A "Developer Mode" tab to view real-time logs from the main shell process.
- **Shortcut Manager:** Configure keyboard shortcuts for opening the Launcher, Quick Settings, etc.

---

## 4. Key Features to Implement

### 4.1 Live Preview
When a user changes a CSS value or a bar position, the shell should apply it immediately. If the user "Cancels" or closes without saving, the shell should revert to the previous state.

### 4.2 Configuration Profiles
Allow users to export and import their entire Axis setup (Themes + Service Settings) as a JSON file or "Profile."

### 4.3 First-Run Wizard
A dedicated sub-page or setup flow for new users:
1. "Welcome to Axis"
2. Continuity Pairing guide.
3. Choosing a default layout.

---

## 5. What to Watch Out For

- **D-Bus Performance:** Avoid flooding D-Bus with too many properties (especially during slider movement). Use debounced updates.
- **Niri Compatibility:** Some settings (like bar position or autohide) require coordination with the niri compositor via IPC.
- **Permissions:** Some hardware settings (Backlight, Battery) might require certain user groups or helper daemons.

---

## 6. Development Roadmap

1. **Step 1:** Expand `org.axis.Shell` D-Bus interface to support "Properties" and "Get/Set" for all services.
2. **Step 2:** Scaffold the GTK4/Libadwaita application with a sidebar/stack navigation.
3. **Step 3:** Implement the **Continuity** page first, as it's the most complex and beneficial.
4. **Step 4:** Add the **Appearance** page with CSS live-reloading.
5. **Step 5:** Finalize the **Bar/General** settings.
