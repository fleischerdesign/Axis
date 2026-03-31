use crate::constants::REVEALER_TRANSITION_MS;
use gtk4::prelude::*;
use std::time::Duration;

/// Factory: create a crossfade revealer with standard transition settings.
pub fn create_revealer() -> gtk4::Revealer {
    gtk4::Revealer::builder()
        .transition_type(gtk4::RevealerTransitionType::Crossfade)
        .transition_duration(250)
        .build()
}

/// Animate a revealer out and remove it from its parent after the transition.
/// Optionally runs a cleanup callback after removal.
pub fn animate_out(
    revealer: &gtk4::Revealer,
    parent: &gtk4::Box,
    cleanup: Option<impl Fn() + 'static>,
) {
    revealer.set_reveal_child(false);
    let rev = revealer.clone();
    let parent = parent.clone();
    gtk4::glib::timeout_add_local_once(
        Duration::from_millis(REVEALER_TRANSITION_MS as u64),
        move || {
            parent.remove(&rev);
            if let Some(f) = cleanup {
                f();
            }
        },
    );
}
