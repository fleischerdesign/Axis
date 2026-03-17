pub mod launcher;
pub mod center;
pub mod status;

use crate::app_context::AppContext;
use launcher::BarLauncher;
use center::BarCenter;
use status::BarStatus;

use gtk4::glib;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

const BAR_HEIGHT: i32 = 54;
const PEEK_PX: i32 = 1;
const HIDE_DELAY_MS: u64 = 300;
const ANIM_INTERVAL_MS: u64 = 16;
const ANIM_STEP: i32 = 8;

#[derive(Clone)]
pub struct Bar {
    pub window: gtk4::ApplicationWindow,
    pub launcher_island: gtk4::Box,
    pub status_island: gtk4::Box,
    pub center_island: gtk4::Box,
    pub vol_icon: gtk4::Image,
    pub popup_open: Rc<RefCell<bool>>,
    is_visible: Rc<RefCell<bool>>,
    hide_timeout: Rc<RefCell<Option<glib::SourceId>>>,
    anim_source: Rc<RefCell<Option<glib::SourceId>>>,
    is_hovered: Rc<RefCell<bool>>,
}

impl Bar {
    pub fn new(app: &libadwaita::Application, ctx: AppContext) -> Self {
        let is_visible = Rc::new(RefCell::new(false));
        let hide_timeout = Rc::new(RefCell::new(None));
        let anim_source = Rc::new(RefCell::new(None));
        let popup_open = Rc::new(RefCell::new(false));
        let is_hovered = Rc::new(RefCell::new(false));

        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("Carp Bottom Bar")
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        window.set_exclusive_zone(-1);
        window.set_margin(Edge::Bottom, -(BAR_HEIGHT - PEEK_PX));

        // Sub-Komponenten initialisieren
        let launcher = BarLauncher::new();
        let center = BarCenter::new(ctx.clone());
        let status = BarStatus::new(ctx.clone());

        let root = gtk4::CenterBox::new();
        root.set_margin_bottom(10);
        root.set_height_request(44);
        root.set_start_widget(Some(&launcher.container));
        root.set_center_widget(Some(&center.container));
        root.set_end_widget(Some(&status.container));
        window.set_child(Some(&root));

        let bar = Self {
            window,
            launcher_island: launcher.container,
            status_island: status.container,
            center_island: center.container,
            vol_icon: status.vol_icon,
            popup_open: popup_open.clone(),
            is_visible: is_visible.clone(),
            hide_timeout: hide_timeout.clone(),
            anim_source: anim_source.clone(),
            is_hovered: is_hovered.clone(),
        };

        // --- AUTO-HIDE ---
        let motion = gtk4::EventControllerMotion::new();
        {
            let is_hovered_c = is_hovered.clone();
            let is_visible_c = is_visible.clone();
            let hide_timeout_c = hide_timeout.clone();
            let anim_source_c = anim_source.clone();
            let window_c = bar.window.clone();

            motion.connect_enter(move |_, _, _| {
                *is_hovered_c.borrow_mut() = true;
                if let Some(src) = hide_timeout_c.borrow_mut().take() { src.remove(); }
                if *is_visible_c.borrow() || anim_source_c.borrow().is_some() { return; }
                *is_visible_c.borrow_mut() = true;

                let window_anim = window_c.clone();
                let anim_source_cb = anim_source_c.clone();
                let src = glib::timeout_add_local(Duration::from_millis(ANIM_INTERVAL_MS), move || {
                    let current = window_anim.margin(Edge::Bottom);
                    let next = (current + ANIM_STEP).min(0);
                    window_anim.set_margin(Edge::Bottom, next);
                    if next >= 0 {
                        *anim_source_cb.borrow_mut() = None;
                        glib::ControlFlow::Break
                    } else { glib::ControlFlow::Continue }
                });
                *anim_source_c.borrow_mut() = Some(src);
            });
        }

        {
            let is_hovered_c = is_hovered.clone();
            let bar_instance = bar.clone();
            motion.connect_leave(move |_| {
                *is_hovered_c.borrow_mut() = false;
                bar_instance.check_auto_hide();
            });
        }
        bar.window.add_controller(motion);

        bar
    }

    pub fn check_auto_hide(&self) {
        if *self.popup_open.borrow() || *self.is_hovered.borrow() { return; }
        if let Some(src) = self.hide_timeout.borrow_mut().take() { src.remove(); }

        let is_visible_for_cb = self.is_visible.clone();
        let hide_timeout_for_cb = self.hide_timeout.clone();
        let anim_source_for_cb = self.anim_source.clone();
        let window_anim = self.window.clone();

        let src = glib::timeout_add_local_once(Duration::from_millis(HIDE_DELAY_MS), move || {
            *is_visible_for_cb.borrow_mut() = false;
            *hide_timeout_for_cb.borrow_mut() = None;
            if let Some(anim) = anim_source_for_cb.borrow_mut().take() { anim.remove(); }
            let anim_source_cb = anim_source_for_cb.clone();
            let src = glib::timeout_add_local(Duration::from_millis(ANIM_INTERVAL_MS), move || {
                let current = window_anim.margin(Edge::Bottom);
                let target = -(BAR_HEIGHT - PEEK_PX);
                let next = (current - ANIM_STEP).max(target);
                window_anim.set_margin(Edge::Bottom, next);
                if next <= target {
                    *anim_source_cb.borrow_mut() = None;
                    glib::ControlFlow::Break
                } else { glib::ControlFlow::Continue }
            });
            *anim_source_for_cb.borrow_mut() = Some(src);
        });
        *self.hide_timeout.borrow_mut() = Some(src);
    }
}
