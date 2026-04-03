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
            log::debug!("[sync] service→config SKIPPED (no change, enabled={})", enabled);
            return; // no change — skip to prevent feedback loop
        }
        log::info!("[sync] service→config SENDING UpdatePartial enabled={}", enabled);
        *last_enabled.borrow_mut() = Some(enabled);
        let _ = settings_stx.try_send(SettingsCmd::UpdatePartial(
            Box::new(move |cfg| config_set(cfg, enabled))
        ));
    });
}

/// Specialized sync for Continuity: handles both the global 'enabled' flag
/// and the 'peer_configs' map (arrangements).
pub fn wire_continuity_sync(
    settings_store: &ServiceStore<super::SettingsData>,
    cont_store: &ServiceStore<crate::services::continuity::ContinuityData>,
    cont_tx: &Sender<crate::services::continuity::ContinuityCmd>,
    settings_tx: &Sender<SettingsCmd>,
) {
    use crate::services::continuity::{ContinuityCmd, PeerArrangement, Side, PeerConfig};
    use super::config::{ArrangementSide, PeerPersistedConfig};
    use std::collections::HashMap;

    // ── Config → Service ──────────────────────────────────────────────────
    let ctx = cont_tx.clone();
    let c_store = cont_store.clone();
    settings_store.subscribe(move |data| {
        let cfg = &data.config.continuity;
        let current = c_store.get();

        // 1. Enabled state
        if cfg.enabled != current.enabled {
            let _ = ctx.try_send(ContinuityCmd::SetEnabled(cfg.enabled));
        }

        // 2. Peer Configs (Arrangements)
        let mut service_update = HashMap::new();
        for p_cfg in &cfg.peer_configs {
            let side = match p_cfg.arrangement_side {
                ArrangementSide::Left => Side::Left,
                ArrangementSide::Right => Side::Right,
                ArrangementSide::Top => Side::Top,
                ArrangementSide::Bottom => Side::Bottom,
            };
            let offset = match side {
                Side::Left | Side::Right => p_cfg.arrangement_y,
                Side::Top | Side::Bottom => p_cfg.arrangement_x,
            };

            service_update.insert(p_cfg.device_id.clone(), PeerConfig {
                trusted: p_cfg.trusted,
                arrangement: PeerArrangement { side, offset },
                clipboard: p_cfg.clipboard,
                audio: p_cfg.audio,
                drag_drop: p_cfg.drag_drop,
                version: p_cfg.version,
            });
        }

        // Only send update if different from what service has
        if service_update != current.peer_configs {
            let _ = ctx.try_send(ContinuityCmd::UpdatePeerConfigs(service_update));
        }
    });

    // ── Service → Config ──────────────────────────────────────────────────
    let s_tx = settings_tx.clone();
    let last_state: Rc<RefCell<Option<(bool, Vec<(String, Side, i32, bool, bool, bool, u64, bool)>)>>> = Rc::new(RefCell::new(None));
    
    cont_store.subscribe(move |data| {
        let mut current_peers: Vec<(String, Side, i32, bool, bool, bool, u64, bool)> = data.peer_configs.iter()
            .map(|(id, cfg)| (
                id.clone(), 
                cfg.arrangement.side, 
                cfg.arrangement.offset,
                cfg.clipboard,
                cfg.audio,
                cfg.drag_drop,
                cfg.version,
                cfg.trusted
            ))
            .collect();
        // Sort for stable comparison
        current_peers.sort_by(|a, b| a.0.cmp(&b.0));

        let state = (data.enabled, current_peers.clone());
        if last_state.borrow().as_ref() == Some(&state) {
            return;
        }
        *last_state.borrow_mut() = Some(state);

        log::debug!("[sync] continuity→config: diff detected (enabled={}, peers={})", data.enabled, current_peers.len());
        
        let enabled = data.enabled;
        let peer_updates = current_peers;

        let _ = s_tx.try_send(SettingsCmd::UpdatePartial(Box::new(move |cfg| {
            cfg.continuity.enabled = enabled;
            
            for (id, side, offset, clipboard, audio, drag_drop, version, trusted) in peer_updates {
                let a_side = match side {
                    Side::Left => ArrangementSide::Left,
                    Side::Right => ArrangementSide::Right,
                    Side::Top => ArrangementSide::Top,
                    Side::Bottom => ArrangementSide::Bottom,
                };
                let (ax, ay) = match side {
                    Side::Left | Side::Right => (0, offset),
                    Side::Top | Side::Bottom => (offset, 0),
                };

                if let Some(p) = cfg.continuity.peer_configs.iter_mut().find(|p| p.device_id == id) {
                    if p.arrangement_side != a_side 
                        || p.arrangement_x != ax 
                        || p.arrangement_y != ay 
                        || p.clipboard != clipboard
                        || p.audio != audio
                        || p.drag_drop != drag_drop
                        || p.version != version
                        || p.trusted != trusted
                    {
                        p.arrangement_side = a_side;
                        p.arrangement_x = ax;
                        p.arrangement_y = ay;
                        p.clipboard = clipboard;
                        p.audio = audio;
                        p.drag_drop = drag_drop;
                        p.version = version;
                        p.trusted = trusted;
                    }
                } else {
                    cfg.continuity.peer_configs.push(PeerPersistedConfig {
                        device_id: id,
                        device_name: "New Peer".to_string(),
                        trusted,
                        arrangement_side: a_side,
                        arrangement_x: ax,
                        arrangement_y: ay,
                        clipboard,
                        audio,
                        drag_drop,
                        version,
                    });
                }
            }
        })));
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
