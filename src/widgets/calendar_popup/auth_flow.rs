use crate::app_context::AppContext;
use crate::services::tasks::{AuthStatus, TaskProvider};
use gtk4::prelude::*;

pub fn show_auth_prompt(
    auth_box: &gtk4::Box,
    ctx: &AppContext,
) {
    while let Some(child) = auth_box.first_child() {
        auth_box.remove(&child);
    }
    auth_box.set_visible(true);

    let mut registry = ctx.task_registry.lock().unwrap();
    match registry.active_mut().auth_status() {
        AuthStatus::NeedsAuth { .. } => {
            drop(registry);
            show_start_button(auth_box, ctx);
        }
        AuthStatus::Failed(msg) => {
            drop(registry);
            log::warn!("[calendar] Auth failed: {msg}");
            let empty = gtk4::Label::builder()
                .label("Anmeldung fehlgeschlagen")
                .css_classes(vec!["calendar-empty".to_string()])
                .halign(gtk4::Align::Start)
                .margin_top(8)
                .build();
            auth_box.append(&empty);
        }
        AuthStatus::Authenticated => {
            drop(registry);
        }
    }
}

fn show_start_button(auth_box: &gtk4::Box, ctx: &AppContext) {
    let info_label = gtk4::Label::builder()
        .label("Google Tasks erfordert Anmeldung")
        .css_classes(vec!["calendar-auth-info".to_string()])
        .halign(gtk4::Align::Center)
        .margin_bottom(8)
        .build();
    auth_box.append(&info_label);

    let start_btn = gtk4::Button::builder()
        .label("Anmelden")
        .css_classes(vec!["calendar-auth-btn".to_string()])
        .halign(gtk4::Align::Center)
        .build();

    let ctx_c = ctx.clone();
    start_btn.connect_clicked(move |btn| {
        btn.set_sensitive(false);
        btn.set_label("Warte...");

        log::info!("[calendar] Starting auth flow...");
        let mut registry = ctx_c.task_registry.lock().unwrap();
        match registry.active_mut().authenticate() {
            Ok(_) => {
                log::info!("[calendar] Auth complete!");
                drop(registry);
                btn.set_label("Angemeldet! Neustarten...");
            }
            Err(e) => {
                log::warn!("[calendar] Auth failed: {e}");
                btn.set_sensitive(true);
                btn.set_label("Fehler — Nochmal versuchen");
            }
        }
    });
    auth_box.append(&start_btn);
}
