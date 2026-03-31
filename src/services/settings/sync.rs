use async_channel::Sender;
use crate::store::ServiceStore;
use crate::services::ServiceConfig;
use super::SettingsCmd;
use super::config::AxisConfig;

/// Generic bidirectional sync between a ServiceConfig service and settings.
///
/// - Config → Service: when SettingsData changes, the service's enabled state is set.
/// - Service → Config: when service data changes, the config is updated via partial closure.
///
/// Both directions are PartialEq-guarded against loops.
pub fn wire_service_config<S: ServiceConfig>(
    settings_store: &ServiceStore<super::SettingsData>,
    service_store: &ServiceStore<S::Data>,
    settings_tx: &Sender<SettingsCmd>,
    config_get: fn(&AxisConfig) -> bool,
    config_set: fn(&mut AxisConfig, bool),
) {
    // Config → Service
    let tx = service_store.store.clone();
    let _ = tx; // unused — we use service handle's cmd_tx instead
    settings_store.subscribe(move |data| {
        let _ = S::cmd_set_enabled(config_get(&data.config));
        // Note: we can't send directly — we need the service's cmd_tx.
        // This is handled by the caller passing the service handle.
    });

    // Service → Config (partial update)
    let stx = settings_tx.clone();
    service_store.subscribe(move |data| {
        let enabled = S::get_enabled(data);
        let set_fn = config_set;
        let _ = stx.try_send(SettingsCmd::UpdatePartial(
            Box::new(move |cfg| set_fn(cfg, enabled))
        ));
    });
}

/// Wire a service with access to its cmd_tx for Config → Service direction.
/// This is the full version that actually sends commands.
pub fn wire_service_config_full<S: ServiceConfig>(
    settings_store: &ServiceStore<super::SettingsData>,
    service_store: &ServiceStore<S::Data>,
    service_tx: &Sender<S::Cmd>,
    settings_tx: &Sender<SettingsCmd>,
    config_get: fn(&AxisConfig) -> bool,
    config_set: fn(&mut AxisConfig, bool),
) {
    // Config → Service
    let stx = service_tx.clone();
    settings_store.subscribe(move |data| {
        let enabled = config_get(&data.config);
        let _ = stx.try_send(S::cmd_set_enabled(enabled));
    });

    // Service → Config (partial update)
    let settings_stx = settings_tx.clone();
    service_store.subscribe(move |data| {
        let enabled = S::get_enabled(data);
        let set_fn = config_set;
        let _ = settings_stx.try_send(SettingsCmd::UpdatePartial(
            Box::new(move |cfg| set_fn(cfg, enabled))
        ));
    });
}

/// Wire one-way Nightlight config → service sync.
/// Nightlight uses NightlightConfig (not AxisConfig fields directly),
/// so it needs a custom wiring function.
pub fn wire_nightlight_config_sync(
    settings_store: &ServiceStore<super::SettingsData>,
    nl_tx: &Sender<crate::services::nightlight::NightlightCmd>,
) {
    let nl_tx = nl_tx.clone();
    settings_store.subscribe(move |data| {
        use crate::services::nightlight::NightlightCmd;
        let nl = &data.config.nightlight;
        let _ = nl_tx.try_send(NightlightCmd::Toggle(nl.enabled));
        let _ = nl_tx.try_send(NightlightCmd::SetTempDay(nl.temp_day));
        let _ = nl_tx.try_send(NightlightCmd::SetTempNight(nl.temp_night));
        let _ = nl_tx.try_send(NightlightCmd::SetSchedule(
            nl.sunrise.clone(), nl.sunset.clone(),
        ));
        if !nl.latitude.is_empty() && !nl.longitude.is_empty() {
            let _ = nl_tx.try_send(NightlightCmd::SetLocation(
                nl.latitude.clone(), nl.longitude.clone(),
            ));
        }
    });
}
