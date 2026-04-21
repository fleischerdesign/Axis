use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use gtk4::{glib, gio};
use gtk4_layer_shell::{LayerShell, Layer, Edge};
use std::sync::Arc;
use std::rc::Rc;

use crate::widgets::island::Island;
use crate::widgets::status_bar::StatusBar;
use crate::widgets::clock::ClockWidget;
use crate::widgets::audio::AudioWidget;
use crate::widgets::workspace_dots::WorkspaceDots;
use crate::widgets::launcher::LauncherWidget;
use crate::widgets::bar::Bar;
use crate::widgets::tray::TrayWidget;

use crate::presentation::battery::BatteryPresenter;
use crate::presentation::clock::ClockPresenter;
use crate::presentation::audio::AudioPresenter;
use crate::presentation::workspaces::WorkspacePresenter;
use crate::presentation::auto_hide::{AutoHidePresenter, AutoHideView};
use crate::presentation::tray::{TrayPresenter, TrayView};

use axis_application::use_cases::popups::TogglePopupUseCase;
use axis_domain::models::popups::PopupType;

glib::wrapper! {
    pub struct BarWindow(ObjectSubclass<imp::BarWindow>)
        @extends gtk4::Widget, gtk4::Window, gtk4::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl BarWindow {
    pub fn new(app: &libadwaita::Application) -> Self {
        glib::Object::builder()
            .property("application", app)
            .build()
    }

    pub fn setup_content(
        &self, 
        battery_presenter: Arc<BatteryPresenter>,
        clock_presenter: Arc<ClockPresenter>,
        audio_presenter: Rc<AudioPresenter>,
        workspace_presenter: Arc<WorkspacePresenter>,
        auto_hide_presenter: Arc<AutoHidePresenter>,
        tray_presenter: Rc<TrayPresenter>,
        toggle_popup_use_case: Arc<TogglePopupUseCase>,
    ) {
        let bar = Bar::new();
        bar.set_vexpand(true);

        // 1. Start Island: Launcher
        let launcher_island = Island::new();
        let launcher_widget = LauncherWidget::new();
        launcher_island.append(&launcher_widget);
        
        let tp = toggle_popup_use_case.clone();
        launcher_island.on_clicked(move || {
            let uc = tp.clone();
            tokio::spawn(async move { let _ = uc.execute(PopupType::Launcher).await; });
        });
        bar.set_start_widget(Some(&launcher_island));

        // 2. Center Island: Clock & Workspaces
        let center_island_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        center_island_box.set_halign(gtk4::Align::Center);

        let ws_island = Island::new();
        let ws_dots = WorkspaceDots::new();
        ws_island.append(&ws_dots);
        
        let tp = toggle_popup_use_case.clone();
        ws_island.on_clicked(move || {
            let uc = tp.clone();
            tokio::spawn(async move { let _ = uc.execute(PopupType::Workspaces).await; });
        });
        center_island_box.append(&ws_island);

        let clock_island = Island::new();
        let clock_widget = ClockWidget::new();
        clock_island.append(&clock_widget);
        
        let tp = toggle_popup_use_case.clone();
        clock_island.on_clicked(move || {
            let uc = tp.clone();
            tokio::spawn(async move { let _ = uc.execute(PopupType::Calendar).await; });
        });
        center_island_box.append(&clock_island);

        bar.set_center_widget(Some(&center_island_box));

        // 3. Tray Island
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

        // 4. End Island: Status (Audio + Battery)
        let end_island = Island::new();
        let audio_widget = AudioWidget::new();
        let status_bar = StatusBar::new();
        end_island.append(&audio_widget);
        end_island.append(&status_bar);
        
        let tp = toggle_popup_use_case.clone();
        end_island.on_clicked(move || {
            let uc = tp.clone();
            tokio::spawn(async move { let _ = uc.execute(PopupType::QuickSettings).await; });
        });

        let end_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        end_box.append(&tray_widget);
        end_box.append(&end_island);
        bar.set_end_widget(Some(&end_box));

        // --- Bind Presenters ---
        battery_presenter.add_view(Box::new(status_bar.clone()));

        let cp = clock_presenter.clone();
        let cv = Box::new(clock_widget.clone());
        glib::spawn_future_local(async move { cp.bind(cv).await; });

        // Multi-View Presenter (Synchron)
        audio_presenter.add_view(Box::new(audio_widget.clone()));

        let wp = workspace_presenter.clone();
        let wv = Box::new(ws_dots.clone());
        glib::spawn_future_local(async move { wp.bind(wv).await; });

        // --- Auto-Hide Setup ---
        let motion = gtk4::EventControllerMotion::new();
        {
            let ahp = auto_hide_presenter.clone();
            let view = self.clone();
            motion.connect_enter(move |_, _, _| { ahp.handle_enter(&view); });
        }
        {
            let ahp = auto_hide_presenter.clone();
            let view = self.clone();
            motion.connect_leave(move |_| { ahp.handle_leave(&view); });
        }
        self.add_controller(motion);

        self.set_margin(Edge::Bottom, auto_hide_presenter.get_initial_margin(54));
        self.set_child(Some(&bar));
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
