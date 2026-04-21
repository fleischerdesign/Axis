# AGENTS.md

## Project Overview
Axis is a desktop shell for the Niri Wayland compositor, built with Rust + GTK4 + Layer Shell.
Architecture: Hexagonal (Domain → Application → Infrastructure → Shell).

## Build Commands
```bash
cargo build                    # Full workspace build
cargo build -p axis-shell      # Build shell binary only
cargo clippy -- -D warnings    # Lint (treat warnings as errors)
cargo clippy                   # Lint (warnings only)
cargo check                    # Quick type-check
```

## Test Commands
```bash
cargo test                     # Run all tests
cargo test -p axis-domain      # Test single crate
```

## Project Structure
```
crates/axis-domain/         → Pure models + port traits (zero external deps)
crates/axis-application/    → Use cases (depends: domain)
crates/axis-infrastructure/ → Adapters + mocks (depends: domain)
crates/axis-shell/          → GTK4 UI (depends: domain + application + infrastructure)
crates/axis-settings/       → Settings app (stub)
backup/                     → Legacy reference implementation
```

## Conventions
- Language: Rust 2024 edition
- Async runtime: Tokio
- GTK thread safety: Use `glib::idle_add_local()` for any GTK mutation from async context
- Adapter pattern: `watch::channel<Status>` + background thread/task, implement domain port trait
- Presenter pattern: `add_view()` + `run_sync()` for multi-view, `bind(view)` for single-view
- No emojis in code. No unnecessary comments.
- Error handling: Use `Result<T, DomainError>`, no panics in production paths
- Logging: `log::info!` / `log::warn!` / `log::error!` (never `println!`)
