use async_channel::Sender;
use crate::store::ServiceStore;
use super::config::ServicesConfig;
use super::SettingsCmd;

/// Wire bidirectional sync between ServicesConfig and a service.
///
/// - Config → Service: when SettingsData changes, `apply` is called with the services config.
/// - Service → Config: when service data changes, `to_partial` returns a closure
///   that mutates ServicesConfig. SettingsService's PartialEq guard prevents loops.
pub fn wire_services_sync<S: Clone + PartialEq + Send + 'static>(
    settings_store: &ServiceStore<super::SettingsData>,
    service_store: &ServiceStore<S>,
    settings_tx: &Sender<SettingsCmd>,
    apply: impl Fn(&ServicesConfig) + 'static,
    to_partial: impl Fn(&S) -> Box<dyn FnOnce(&mut ServicesConfig) + Send> + 'static,
) {
    // Config → Service
    settings_store.subscribe(move |data| {
        apply(&data.config.services);
    });

    // Service → Config (partial update, merged in SettingsService)
    let tx = settings_tx.clone();
    service_store.subscribe(move |data| {
        let partial_fn = to_partial(data);
        let _ = tx.try_send(SettingsCmd::UpdateServicesPartial(partial_fn));
    });
}

/// Wire one-way Nightlight config → service sync.
/// Nightlight uses NightlightConfig (not ServicesConfig), so it needs
/// a separate wiring function.
pub fn wire_nightlight_config_sync(
    settings_store: &ServiceStore<super::SettingsData>,
    nl_tx: &Sender<crate::services::nightlight::NightlightCmd>,
) {
    let nl_tx = nl_tx.clone();
    settings_store.subscribe(move |data| {
        let nl = &data.config.nightlight;
        let _ = nl_tx.try_send(crate::services::nightlight::NightlightCmd::Toggle(nl.enabled));
        let _ = nl_tx.try_send(crate::services::nightlight::NightlightCmd::SetTempDay(nl.temp_day));
        let _ = nl_tx.try_send(crate::services::nightlight::NightlightCmd::SetTempNight(nl.temp_night));
        let _ = nl_tx.try_send(crate::services::nightlight::NightlightCmd::SetSchedule(
            nl.sunrise.clone(), nl.sunset.clone(),
        ));
        if !nl.latitude.is_empty() && !nl.longitude.is_empty() {
            let _ = nl_tx.try_send(crate::services::nightlight::NightlightCmd::SetLocation(
                nl.latitude.clone(), nl.longitude.clone(),
            ));
        }
    });
}

/// Wire Continuity config → service sync.
/// Uses ContinuityConfig (not ServicesConfig).
/// Bidirectional: config changes → service, service changes → config.
pub fn wire_continuity_sync(
    settings_store: &ServiceStore<super::SettingsData>,
    continuity_store: &ServiceStore<crate::services::continuity::ContinuityData>,
    settings_tx: &Sender<SettingsCmd>,
    ct_tx: &Sender<crate::services::continuity::ContinuityCmd>,
) {
    use crate::services::continuity::ContinuityCmd;

    // Config → Service
    let ct_tx_c = ct_tx.clone();
    settings_store.subscribe(move |data| {
        let _ = ct_tx_c.try_send(ContinuityCmd::SetEnabled(data.config.continuity.enabled));
    });

    // Service → Config
    let stx = settings_tx.clone();
    continuity_store.subscribe(move |data| {
        let enabled = data.enabled;
        let _ = stx.try_send(SettingsCmd::UpdateContinuityPartial(
            Box::new(move |cfg| cfg.enabled = enabled)
        ));
    });
}

// ── Concrete Service Sync Registrations ─────────────────────────────────────

use crate::services::dnd::{DndCmd, DndData};
use crate::services::airplane::{AirplaneCmd, AirplaneData};
use crate::services::bluetooth::{BluetoothCmd, BluetoothData};

pub fn wire_dnd_sync(
    settings: &ServiceStore<super::SettingsData>,
    dnd: &ServiceStore<DndData>,
    tx: &Sender<SettingsCmd>,
    dnd_tx: &Sender<DndCmd>,
) {
    let dnd_tx_c = dnd_tx.clone();
    wire_services_sync(settings, dnd, tx,
        move |cfg| { let _ = dnd_tx_c.try_send(DndCmd::Toggle(cfg.dnd_enabled)); },
        |data| {
            let enabled = data.enabled;
            Box::new(move |cfg: &mut ServicesConfig| { cfg.dnd_enabled = enabled; })
        },
    );
}

pub fn wire_airplane_sync(
    settings: &ServiceStore<super::SettingsData>,
    airplane: &ServiceStore<AirplaneData>,
    tx: &Sender<SettingsCmd>,
    airplane_tx: &Sender<AirplaneCmd>,
) {
    let ap_tx = airplane_tx.clone();
    wire_services_sync(settings, airplane, tx,
        move |cfg| { let _ = ap_tx.try_send(AirplaneCmd::Toggle(cfg.airplane_enabled)); },
        |data| {
            let enabled = data.enabled;
            Box::new(move |cfg: &mut ServicesConfig| { cfg.airplane_enabled = enabled; })
        },
    );
}

pub fn wire_bluetooth_sync(
    settings: &ServiceStore<super::SettingsData>,
    bluetooth: &ServiceStore<BluetoothData>,
    tx: &Sender<SettingsCmd>,
    bt_tx: &Sender<BluetoothCmd>,
) {
    let bt_tx_c = bt_tx.clone();
    wire_services_sync(settings, bluetooth, tx,
        move |cfg| { let _ = bt_tx_c.try_send(BluetoothCmd::TogglePower(cfg.bluetooth_enabled)); },
        |data| {
            let powered = data.is_powered;
            Box::new(move |cfg: &mut ServicesConfig| { cfg.bluetooth_enabled = powered; })
        },
    );
}
