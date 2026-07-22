# Axis Desktop Shell

> A modern, high-performance, modular desktop shell and control center for **Wayland compositors** (such as Niri, Hyprland, and Sway), built with **Rust**, **GTK4 / Libadwaita**, and **wlr-layer-shell**.

[![Release](https://img.shields.io/github/v/release/fleischerdesign/Axis)](https://github.com/fleischerdesign/Axis/releases)
[![License](https://img.shields.io/github/license/fleischerdesign/Axis)](LICENSE)
[![CI Status](https://github.com/fleischerdesign/Axis/actions/workflows/ci.yml/badge.svg)](https://github.com/fleischerdesign/Axis/actions/workflows/ci.yml)

---

## Overview

**Axis** is a complete, lightweight desktop environment shell designed for modern Wayland compositors. It consists of two primary binaries:

1. **`axis-shell`:** A Wayland Layer Shell statusbar, system indicator panel, MPRIS media widget, application launcher, and quick-settings popup menu.
2. **`axis-settings`:** A native Libadwaita control center application providing system configuration for appearance, networking, bluetooth, device continuity, accounts, power, and idle behavior.

Axis is architected around **Hexagonal Architecture (Ports and Adapters)**, ensuring pure zero-dependency domain models, decoupled async infrastructure adapters, and maximum testability.

---

## Key Features & Modules

### Axis Shell (`axis-shell`)

- **Wayland Layer Shell Integration:** Native panel overlay positioning via `gtk4-layer-shell`.
- **System Indicators & Quick Settings:** Quick controls for Wi-Fi access points, Bluetooth devices, Audio volume, Night Light, and Power profiles.
- **Optimistic Scan Spinner:** Instant visual feedback when refreshing available Wi-Fi networks or Bluetooth devices.
- **MPRIS Media Player Controls:** Track information, album artwork rendering, play/pause toggles, and volume control.
- **Application Launcher:** Desktop entry parser with fuzzy search and executable dispatch.

### Axis Settings App (`axis-settings`)

- **Appearance Settings:** Responsive `AdwClamp` layout with Light/Dark scheme cards, accent color swatches with active borders, and dynamic wallpaper picture preview.
- **Network Settings:** Connected Wi-Fi network pinning, signal strength indicators, and access point list with immediate scan button integration.
- **Bluetooth Settings:** Device discovery, paired vs. available device groups, status indicators, and empty state pages.
- **Accounts Settings:** User profile avatar integration, account status badges, and re-authentication action rows.
- **Continuity Sync:** Local peer device discovery using mDNS/Avahi, host filtering (`.local` trimming), and cross-device feature management.
- **Idle & Power Settings:** Presentation mode (*Idle Inhibit*) toggle with dynamic UI sensitivity, lock screen timers, screen blanking, and system suspend timeouts.
- **About Settings:** System information (OS, Kernel, Compositor, GTK, libadwaita versions) and direct links to issue reporting and repository source code.

---

## Architecture & Codebase Design

Axis strictly enforces a clean **Hexagonal Architecture** across its workspace crates:

```mermaid
graph TD
    subgraph UI ["UI & Presentation Layer"]
        Shell["crates/axis-shell (GTK4 / Layer Shell)"]
        Settings["crates/axis-settings (Libadwaita App)"]
        Presentation["crates/axis-presentation (Presenters & View Traits)"]
    end

    subgraph Infrastructure ["Infrastructure Adapters (Tokio Async)"]
        Infra["crates/axis-infrastructure"]
        NM["NetworkManager (D-Bus)"]
        BlueZ["BlueZ Bluetooth (D-Bus)"]
        Avahi["Avahi mDNS Continuity"]
        Pulse["PulseAudio / PipeWire"]
        Night["Night Light / Gamma Control"]
    end

    subgraph Application ["Application Layer"]
        App["crates/axis-application (Use Cases & Workflows)"]
    end

    subgraph Domain ["Pure Domain Layer"]
        Dom["crates/axis-domain (Entities, Value Objects & Port Traits)"]
    end

    Shell --> Presentation
    Settings --> Presentation
    Presentation --> App
    App --> Dom
    Infra --> Dom
    Infra --> NM
    Infra --> BlueZ
    Infra --> Avahi
    Infra --> Pulse
    Infra --> Night
```

### Workspace Crate Matrix

| Crate | Layer | Description | Key Dependencies |
|---|---|---|---|
| [`axis-domain`](crates/axis-domain) | Domain | Core domain entities, status models, and port traits. | *Zero external dependencies* |
| [`axis-application`](crates/axis-application) | Application | Application workflows, use cases, and status presenters. | `axis-domain` |
| [`axis-infrastructure`](crates/axis-infrastructure) | Infrastructure | Concrete D-Bus, NetworkManager, BlueZ, mDNS, and PulseAudio adapters. | `axis-domain`, `tokio`, `zbus` |
| [`axis-presentation`](crates/axis-presentation) | Presentation | Generic presenter patterns, multi-view handling, and status binding traits. | `axis-domain` |
| [`axis-shell`](crates/axis-shell) | UI / Shell | Desktop statusbar, quick settings popups, MPRIS controls, and launcher. | `gtk4`, `gtk4-layer-shell`, `axis-infrastructure` |
| [`axis-settings`](crates/axis-settings) | UI / App | Native Libadwaita configuration app for system settings. | `libadwaita`, `gtk4`, `axis-infrastructure` |

---

## Nix & NixOS Integration

Axis provides full Nix Flake integration for building, development environments, and NixOS configuration.

### Building & Checks with Nix

```bash
# Build default packages (axis-shell and axis-settings)
nix build .#default

# Run hermetic flake checks (clippy, formatting, workspace tests, package)
nix flake check

# Format flake.nix according to Nix standards
nix fmt
```

### NixOS System Integration

Axis exports a NixOS module (`nixosModules.default`) in `flake.nix` that automatically configures required system services for Continuity peer sync, clipboard support, and hardware permissions:

```nix
# configuration.nix
{ inputs, ... }: {
  imports = [
    inputs.axis.nixosModules.default
  ];
}
```

The module automatically configures:
- **`services.avahi`:** Enables local mDNS mDNS/DNS-SD discovery for Axis Continuity peer sync.
- **`networking.firewall`:** Opens TCP port `7391` for Axis Continuity communication.
- **`services.udev`:** Configures `/dev/uinput` permissions for system input events.
- **`environment.systemPackages`:** Installs `wl-clipboard` for Wayland clipboard support.

---

## Building & Installation

### Prerequisites

- **GTK4** (`>= 4.12`) & **libadwaita** (`>= 1.4`)
- **gtk4-layer-shell** (`>= 1.0`)
- **PulseAudio** / **PipeWire** development headers
- **Rust toolchain** (2024 edition)

### Cargo Build

```bash
# Clone the repository
git clone https://github.com/fleischerdesign/Axis.git
cd Axis

# Build all workspace packages
cargo build --release

# Run Axis Shell
cargo run -p axis-shell

# Run Axis Settings App
cargo run -p axis-settings
```

---

## Development & Code Quality

To maintain codebase standards, run the following verification commands before submitting changes:

```bash
# Workspace unit & integration tests
cargo test

# Clippy lints (warnings treated as errors)
cargo clippy -- -D warnings

# Codebase formatting check
cargo fmt --all -- --check

# Hermetic Nix sandbox verification
nix flake check
```

---

## License

Axis is open-source software licensed under the terms of the [GPL-3.0 License](LICENSE).
