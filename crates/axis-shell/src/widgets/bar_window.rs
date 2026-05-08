use gtk4::{gio, glib};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;

use crate::composition::presenters::Presenters;
use crate::presentation::auto_hide::AutoHideView;
use crate::presentation::tray::TrayView;
use crate::widgets::airplane_status::AirplaneStatusWidget;
use crate::widgets::audio::AudioWidget;
use crate::widgets::bar::Bar;
use crate::widgets::bluetooth_status::BluetoothStatusWidget;
use crate::widgets::clock::ClockWidget;
use crate::widgets::continuity_status::ContinuityStatusWidget;
use crate::widgets::dnd_status::DndStatusWidget;
use crate::widgets::idle_inhibit_status::IdleInhibitStatusWidget;
use crate::widgets::island::Island;
use crate::widgets::launcher::LauncherWidget;
use crate::widgets::mpris_bar::MprisBarWidget;
use crate::widgets::status_bar::StatusBar;
use crate::widgets::tray::TrayWidget;
use crate::widgets::wifi_status::WifiStatusWidget;
use crate::widgets::workspace_dots::WorkspaceDots;
use axis_domain::models::popups::PopupType;
use axis_domain::models::workspaces::WorkspaceStatus;
use axis_presentation::FnView;

glib::wrapper! {
    pub struct BarWindow(ObjectSubclass<imp::BarWindow>)
        @extends gtk4::Widget, gtk4::Window, gtk4::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl BarWindow {
    pub fn new(app: &libadwaita::Application) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    pub fn setup_content(&self, p: &Presenters, show_labels: bool) {
        let battery_presenter = p.battery.clone();
        let clock_presenter = p.clock.clone();
        let audio_presenter = p.audio.clone();
        let workspace_presenter = p.workspace.clone();
        let auto_hide_presenter = p.auto_hide.clone();
        let tray_presenter = p.tray.clone();
        let toggle_popup_use_case = p.toggle_popup.clone();
        let toggle_overview_use_case = p.toggle_overview.clone();
        let network_presenter = p.network.clone();
        let bluetooth_presenter = p.bluetooth.clone();
        let dnd_status_presenter = p.dnd_status.clone();
        let airplane_status_presenter = p.airplane_status.clone();
        let continuity_presenter = p.continuity.clone();
        let idle_inhibit_presenter = p.idle_inhibit_status.clone();
        let mpris_presenter = p.mpris.clone();
        let bar = Bar::new();
        bar.container.set_vexpand(true);

        let launcher_island = Island::new();
        let launcher_widget = LauncherWidget::new();
        launcher_island.container.append(&launcher_widget.container);

        let tp = toggle_popup_use_case.clone();
        launcher_island.on_clicked(move || {
            let uc = tp.clone();
            tokio::spawn(async move {
                if let Err(e) = uc.execute(PopupType::Launcher).await {
                    log::error!("[bar] toggle popup failed: {e}");
                }
            });
        });
        bar.set_start_widget(Some(&launcher_island.container));

        let center_island_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        center_island_box.set_halign(gtk4::Align::Center);

        let ws_island = Island::new();
        let ws_dots = WorkspaceDots::new();
        ws_island.container.append(&ws_dots.container);

        let tou = toggle_overview_use_case.clone();
        ws_island.on_clicked(move || {
            let uc = tou.clone();
            tokio::spawn(async move {
                if let Err(e) = uc.execute().await {
                    log::error!("[bar] toggle overview failed: {e}");
                }
            });
        });
        center_island_box.append(&ws_island.container);

        let clock_island = Island::new();
        let clock_widget = ClockWidget::new();
        clock_island.container.append(&clock_widget.container);

        let tp = toggle_popup_use_case.clone();
        clock_island.on_clicked(move || {
            let uc = tp.clone();
            tokio::spawn(async move {
                if let Err(e) = uc.execute(PopupType::Agenda).await {
                    log::error!("[bar] toggle popup failed: {e}");
                }
            });
        });
        center_island_box.append(&clock_island.container);

        let mpris_bar_widget = MprisBarWidget::new();

        let tp_mpris = toggle_popup_use_case.clone();
        let mp_pp = mpris_presenter.clone();
        let gesture_left = gtk4::GestureClick::new();
        gesture_left.set_button(gtk4::gdk::BUTTON_PRIMARY);
        gesture_left.connect_released(move |_, _, _, _| {
            let uc = tp_mpris.clone();
            tokio::spawn(async move {
                if let Err(e) = uc.execute(PopupType::Mpris).await {
                    log::error!("[bar] toggle popup failed: {e}");
                }
            });
        });
        mpris_bar_widget.container.add_controller(gesture_left);

        let gesture_right = gtk4::GestureClick::new();
        gesture_right.set_button(gtk4::gdk::BUTTON_SECONDARY);
        gesture_right.connect_released(move |_, _, _, _| {
            if let Some(id) = mp_pp.active_player_id() {
                mp_pp.play_pause(&id);
            }
        });
        mpris_bar_widget.container.add_controller(gesture_right);
        mpris_bar_widget
            .container
            .set_cursor_from_name(Some("pointer"));

        center_island_box.append(&mpris_bar_widget.container);

        bar.set_center_widget(Some(&center_island_box));

        let tray_widget = TrayWidget::new();

        let tp_activate = tray_presenter.clone();
        let tp_context = tray_presenter.clone();
        let tp_scroll = tray_presenter.clone();
        tray_widget.on_activate(Box::new(move |bus_name, x, y| {
            tp_activate.activate(bus_name, x, y);
        }));
        tray_widget.on_context_menu(Box::new(move |bus_name, x, y| {
            tp_context.context_menu(bus_name, x, y);
        }));
        tray_widget.on_scroll(Box::new(move |bus_name, delta, orientation| {
            tp_scroll.scroll(bus_name, delta, orientation);
        }));

        tray_presenter.add_view(Box::new(tray_widget.clone()));

        let end_island = Island::new();
        let wifi_widget = WifiStatusWidget::new(show_labels);
        let bt_widget = BluetoothStatusWidget::new();
        let dnd_widget = DndStatusWidget::new();
        let airplane_widget = AirplaneStatusWidget::new();
        let continuity_widget = ContinuityStatusWidget::new();
        let idle_inhibit_widget = IdleInhibitStatusWidget::new();
        let audio_widget = AudioWidget::new(show_labels);
        let status_bar = StatusBar::new(show_labels);
        end_island.container.append(&wifi_widget.container);
        end_island.container.append(&bt_widget.container);
        end_island.container.append(&dnd_widget.container);
        end_island.container.append(&airplane_widget.container);
        end_island.container.append(&continuity_widget.container);
        end_island.container.append(&idle_inhibit_widget.container);
        end_island.container.append(&audio_widget.container);
        end_island.container.append(&status_bar.container);

        let tp = toggle_popup_use_case.clone();
        end_island.on_clicked(move || {
            let uc = tp.clone();
            tokio::spawn(async move {
                if let Err(e) = uc.execute(PopupType::QuickSettings).await {
                    log::error!("[bar] toggle popup failed: {e}");
                }
            });
        });

        let end_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        end_box.append(&tray_widget.container);
        end_box.append(&end_island.container);
        bar.set_end_widget(Some(&end_box));

        {
            let ahp = auto_hide_presenter.clone();
            let bw = self.clone();
            let ov = FnView::new(move |status: &WorkspaceStatus| {
                ahp.set_force_visible(&bw, status.overview_open);
            });
            workspace_presenter.add_view(Box::new(ov));
        }

        network_presenter.add_view(Box::new(wifi_widget.clone()));
        bluetooth_presenter.add_view(Box::new(bt_widget.clone()));
        dnd_status_presenter.add_view(Box::new(dnd_widget.clone()));
        airplane_status_presenter.add_view(Box::new(airplane_widget.clone()));
        continuity_presenter.add_view(Box::new(continuity_widget.clone()));
        idle_inhibit_presenter.add_view(Box::new(idle_inhibit_widget.clone()));
        audio_presenter.add_view(Box::new(audio_widget.clone()));
        battery_presenter.add_view(Box::new(status_bar.clone()));

        workspace_presenter.add_view(Box::new(ws_dots.clone()));
        clock_presenter.add_view(Box::new(clock_widget.clone()));
        mpris_presenter.add_view(Box::new(mpris_bar_widget.clone()));

        let motion = gtk4::EventControllerMotion::new();
        {
            let ahp = auto_hide_presenter.clone();
            let view = self.clone();
            motion.connect_enter(move |_, _, _| {
                ahp.handle_enter(&view);
            });
        }
        {
            let ahp = auto_hide_presenter.clone();
            let view = self.clone();
            motion.connect_leave(move |_| {
                ahp.handle_leave(&view);
            });
        }
        self.add_controller(motion);

        self.set_margin(Edge::Bottom, auto_hide_presenter.get_initial_margin(54));
        self.set_child(Some(&bar.container));
    }
}

impl AutoHideView for BarWindow {
    fn set_visible_state(&self, _is_visible: bool) {}
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct BarWindow;

    #[glib::object_subclass]
    impl ObjectSubclass for BarWindow {
        const NAME: &'static str = "BarWindow";
        type Type = super::BarWindow;
        type ParentType = gtk4::ApplicationWindow;
    }

    impl ObjectImpl for BarWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            obj.init_layer_shell();
            obj.set_layer(Layer::Top);
            obj.set_namespace(Some("axis-shell"));

            obj.set_anchor(Edge::Bottom, true);
            obj.set_anchor(Edge::Left, true);
            obj.set_anchor(Edge::Right, true);

            obj.set_exclusive_zone(-1);

            obj.add_css_class("bar-window");
        }
    }

    impl WidgetImpl for BarWindow {}
    impl WindowImpl for BarWindow {}
    impl ApplicationWindowImpl for BarWindow {}
}
