use std::cell::RefCell;
use std::rc::Rc;
use async_channel::Sender;
use crate::store::ServiceStore;
use crate::services::ServiceConfig;
use super::SettingsCmd;
use super::config::AxisConfig;

/// Generic bidirectional sync between a ServiceConfig service and settings.
///
/// - Config → Service: when SettingsData changes, send a command only if the
///   desired enabled state differs from the service's current state.
/// - Service → Config: when service data changes, write config only if the
///   enabled value actually changed (last_enabled guard).
pub fn wire_service_config_full<S: ServiceConfig>(
    settings_store: &ServiceStore<super::SettingsData>,
    service_store: &ServiceStore<S::Data>,
    service_tx: &Sender<S::Cmd>,
    settings_tx: &Sender<SettingsCmd>,
    config_get: fn(&AxisConfig) -> bool,
    config_set: fn(&mut AxisConfig, bool),
) {
    // Config → Service: only send a command when the desired state differs
    // from the service's current state. Without this guard, every SettingsData
    // update (e.g. from UpdatePartial written by Service→Config) would re-send
    // the command even if the service is already in the correct state.
    let stx = service_tx.clone();
    let svc_store_c = service_store.clone();
    settings_store.subscribe(move |data| {
        let desired = config_get(&data.config);
        let current = S::get_enabled(&svc_store_c.get());
        if desired != current {
            let _ = stx.try_send(S::cmd_set_enabled(desired));
        }
    });

    // Service → Config: only write config when the enabled value actually changes.
    // This is the critical loop-prevention guard: without it, every BluetoothData
    // update (e.g. device list change) would trigger an UpdatePartial → file save
    // → file watcher → Reload → Config→Service command → service update → loop.
    let settings_stx = settings_tx.clone();
    let last_enabled: Rc<RefCell<Option<bool>>> = Rc::new(RefCell::new(None));
    service_store.subscribe(move |data| {
        let enabled = S::get_enabled(data);
        if *last_enabled.borrow() == Some(enabled) {
            log::info!("[sync] service→config SKIPPED (no change, enabled={})", enabled);
            return; // no change — skip to prevent feedback loop
        }
        log::info!("[sync] service→config SENDING UpdatePartial enabled={}", enabled);
        *last_enabled.borrow_mut() = Some(enabled);
        let _ = settings_stx.try_send(SettingsCmd::UpdatePartial(
            Box::new(move |cfg| config_set(cfg, enabled))
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
