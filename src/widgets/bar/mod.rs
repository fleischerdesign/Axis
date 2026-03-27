pub mod center;
pub mod launcher;
pub mod status;
pub mod tray;

use crate::app_context::AppContext;
use crate::constants::{BAR_HEIGHT, BAR_HIDE_DELAY_MS, BAR_PEEK_PX};
use crate::store::ReactiveBool;
use crate::widgets::animations::SlideAnimator;
use center::BarCenter;
use launcher::BarLauncher;
use status::BarStatus;
use tray::BarTray;

use gtk4::glib;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

#[derive(Clone)]
pub struct Bar {
    pub window: gtk4::ApplicationWindow,
    pub popup_open: ReactiveBool,
    launcher: gtk4::Box,
    status: gtk4::Box,
    ws: gtk4::Box,
    clock: gtk4::Box,
    vol_icon: gtk4::Image,
    is_visible: ReactiveBool,
    hide_timeout: Rc<RefCell<Option<glib::SourceId>>>,
    anim_source: Rc<RefCell<Option<glib::SourceId>>>,
    is_hovered: ReactiveBool,
}

impl Bar {
    pub fn new(app: &libadwaita::Application, ctx: AppContext) -> Self {
        let is_visible = ReactiveBool::new(false);
        let hide_timeout = Rc::new(RefCell::new(None));
        let anim_source = Rc::new(RefCell::new(None));
        let popup_open = ReactiveBool::new(false);
        let is_hovered = ReactiveBool::new(false);

        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .title("AXIS Bottom Bar")
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Top);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        window.set_exclusive_zone(-1);
        window.set_margin(Edge::Bottom, -(BAR_HEIGHT - BAR_PEEK_PX));

        let launcher = BarLauncher::new();
        let center = BarCenter::new(ctx.clone());
        let status = BarStatus::new(ctx.clone());
        let tray = BarTray::new(ctx.clone());

        let end_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        end_box.append(&tray.container);
        end_box.append(&status.container);

        let root = gtk4::CenterBox::new();
        root.set_margin_bottom(10);
        root.set_height_request(44);
        root.set_start_widget(Some(&launcher.container));
        root.set_center_widget(Some(&center.container));
        root.set_end_widget(Some(&end_box));
        window.set_child(Some(&root));

        let bar = Self {
            window: window.clone(),
            popup_open: popup_open.clone(),
            launcher: launcher.container,
            status: status.container,
            ws: center.ws_island,
            clock: center.clock_island,
            vol_icon: status.vol_icon,
            is_visible: is_visible.clone(),
            hide_timeout: hide_timeout.clone(),
            anim_source: anim_source.clone(),
            is_hovered: is_hovered.clone(),
        };

        // --- AUTO-HIDE ---
        let motion = gtk4::EventControllerMotion::new();
        {
            let is_hovered_c = is_hovered.clone();
            let bar_ref = bar.clone();

            motion.connect_enter(move |_, _, _| {
                is_hovered_c.set(true);
                bar_ref.check_auto_hide();
            });
        }

        {
            let is_hovered_c = is_hovered.clone();
            let bar_ref = bar.clone();
            motion.connect_leave(move |_| {
                is_hovered_c.set(false);
                bar_ref.check_auto_hide();
            });
        }
        window.add_controller(motion);

        bar
    }

    pub fn launcher_island(&self) -> &gtk4::Box {
        &self.launcher
    }

    pub fn status_island(&self) -> &gtk4::Box {
        &self.status
    }

    pub fn workspace_island(&self) -> &gtk4::Box {
        &self.ws
    }

    pub fn clock_island(&self) -> &gtk4::Box {
        &self.clock
    }

    pub fn volume_icon(&self) -> &gtk4::Image {
        &self.vol_icon
    }

    pub fn check_auto_hide(&self) {
        let should_be_visible = self.popup_open.get() || self.is_hovered.get();

        if let Some(src) = self.hide_timeout.borrow_mut().take() {
            src.remove();
        }

        if should_be_visible {
            if !self.is_visible.get() {
                self.is_visible.set(true);
                SlideAnimator::slide_margin(
                    &self.window,
                    Edge::Bottom,
                    0,
                    self.anim_source.clone(),
                );
            }
        } else {
            let is_visible_for_cb = self.is_visible.clone();
            let hide_timeout_for_cb = self.hide_timeout.clone();
            let anim_source_for_cb = self.anim_source.clone();
            let window_anim = self.window.clone();

            let src =
                glib::timeout_add_local_once(Duration::from_millis(BAR_HIDE_DELAY_MS), move || {
                    is_visible_for_cb.set(false);
                    *hide_timeout_for_cb.borrow_mut() = None;

                    SlideAnimator::slide_margin(
                        &window_anim,
                        Edge::Bottom,
                        -(BAR_HEIGHT - BAR_PEEK_PX),
                        anim_source_for_cb,
                    );
                });
            *self.hide_timeout.borrow_mut() = Some(src);
        }
    }
}
