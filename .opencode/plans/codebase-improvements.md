# Codebase Improvements — Axis Shell

Audit-Datum: 2026-04-04

---

## Phase 1: Quick Wins (geringer Aufwand, hoher Impact)

### D-1: `authenticate_async` löschen (31 Zeilen toter Code)
- **Datei:** `crates/axis-core/src/services/google/auth_flow.rs`
- **Änderung:** Datei löschen, Aufrufer auf `GoogleAuthRegistry::authenticate` umstellen

### D-2: `to_pulse_volume()` Helper in audio.rs
- **Datei:** `crates/axis-core/src/services/audio.rs`
- **Änderung:** Duplizierte Volume-Konversion in einen Helper extrahieren

### D-3: `last_day_of_month` durch chrono built-in ersetzen
- **Datei:** `crates/axis-core/src/services/calendar/mod.rs`
- **Änderung:** Manuelle Leap-Year-Logik durch chrono ersetzen

### D-4: `CalendarEvent::format_time_range` panic-safe machen
- **Datei:** `crates/axis-core/src/services/calendar/provider.rs:22-23`
- **Änderung:** `&s[..5]` → `s.get(..5).unwrap_or(s)`

### D-5: `config_dir()` panic → graceful fallback
- **Datei:** `crates/axis-core/src/services/continuity/known_peers.rs:9`
- **Änderung:** `.expect("HOME must be set")` → `.unwrap_or_else(\|_\| "/tmp".into())`

### D-6: Magic Numbers → benannte Konstanten
- **Dateien:** Mehrere
- **Änderung:** `40.0` buffer, `5` retry, `15` max APs, `Volume::NORMAL * 2`, `10` scan retries

### D-7: `TrayService::add_item` → `fetch_and_emit` wiederverwenden
- **Datei:** `crates/axis-core/src/services/tray/mod.rs:160-166`

### D-8: Deutsche Kommentare → Englisch
- **Dateien:** `store.rs`, `backlight.rs`, `launcher/provider.rs`, `apps.rs`, etc.

### D-9: `config_dir()` in allen Modulen vereinheitlichen
- **Dateien:** `tasks/utils.rs`, `continuity/known_peers.rs`, `settings/config.rs`

---

## Phase 2: Best Practice Violations

### C-1: `build_http_client()` `.expect()` → `Result`
- **Datei:** `crates/axis-core/src/services/tasks/utils.rs:50`

### C-3: `AvahiDiscovery::register` `.unwrap()` → Error Handling
- **Datei:** `crates/axis-core/src/services/continuity/discovery.rs:57`

### C-5: `write_accent_css` Errors loggen
- **Datei:** `crates/axis-shell/src/main.rs:581-599`

### C-6: `TrayService` Fetch-Duplikat entfernen
- **Datei:** `crates/axis-core/src/services/tray/mod.rs`

### C-7: `AudioService` Volume-Konversion deduplizieren
- **Datei:** `crates/axis-core/src/services/audio.rs`

---

## Phase 3: Code-Qualität (DRY, Naming, Größe)

### B-1: Duplicated `fetch_data` Pattern in Bluetooth, Network, Power
- **Dateien:** `bluetooth.rs:610-695`, `network.rs:251-304`, `power.rs:34-44`
- **Änderung:** Gemeinsamen D-Bus-Service-Helper extrahieren

### B-2: Google Auth Duplikate in Calendar/Tasks Providern
- **Dateien:** `calendar/google.rs:40-59`, `tasks/google.rs:40-58`
- **Änderung:** Gemeinsames `GoogleAuthProvider` Trait/Struct

### B-4: `MainPage::new` decomposen (329 Zeilen)
- **Datei:** `crates/axis-shell/src/widgets/quick_settings/main_page.rs:32-329`

### B-5: Magic Numbers (siehe D-6)

### B-6: Kommentare auf Englisch (siehe D-8)

### B-7: Import-Reihenfolge standardisieren
- **Regel:** std → external crates → crate modules

---

## Phase 4: Architektur (SOLID, Modularität)

### A-1: `ContinuityInner` decomposen
- **Datei:** `crates/axis-core/src/services/continuity/mod.rs`
- **Änderung:** `PairingManager`, `SharingManager`, `ConnectionManager` extrahieren

### A-2: `GoogleAuthRegistry` aufsplitten
- **Datei:** `crates/axis-core/src/services/google/mod.rs`
- **Änderung:** `GoogleCredentialStore`, `GoogleTokenManager`, `OAuthFlowExecutor`

### A-3: `AppContext` aufräumen
- **Datei:** `crates/axis-shell/src/app_context.rs`
- **Änderung:** `Arc<Mutex<Registry>>` → `ServiceHandle` Pattern

### A-4: `CalendarRegistry` Provider-Injection
- **Datei:** `crates/axis-core/src/services/calendar/mod.rs`
- **Änderung:** `CalendarRegistry::with_provider()` Constructor

### A-5: `TaskRegistry` `is_async()` → Trait-basierter Dispatch
- **Datei:** `crates/axis-core/src/services/tasks/mod.rs`
- **Änderung:** Default-Implementierungen im `TaskProvider` Trait

### A-6: `ContinuityInner::handle_cmd` Parameter bündeln
- **Datei:** `crates/axis-core/src/services/continuity/mod.rs:476-507`
- **Änderung:** `ContinuitySubsystems` Struct

### A-7: `SettingsDbusServer` Macro
- **Datei:** `crates/axis-core/src/services/settings/dbus.rs`
- **Änderung:** `define_dbus_section!` Macro für get/set Paare

### A-8: `wire_continuity_sync` 8-Tuple → benannte Struct
- **Datei:** `crates/axis-core/src/services/settings/sync.rs:113-192`
- **Änderung:** `PeerSyncSnapshot` Struct

---
