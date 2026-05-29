use crate::presentation::audio::{AudioPresenter, audio_icon};
use crate::presentation::battery::BatteryPresenter;
use crate::presentation::bluetooth::BluetoothPresenter;
use crate::presentation::brightness::BrightnessPresenter;
use crate::presentation::network::NetworkPresenter;
use crate::presentation::nightlight::NightlightPresenter;
use crate::presentation::notifications::NotificationPresenter;
use crate::presentation::popups::PopupView;
use crate::presentation::toggle::TogglePresenter;
use crate::widgets::callback::FnCell0;
use crate::widgets::components::battery_button::BatteryButton;
use crate::widgets::components::power_actions::PowerActionStack;
use crate::widgets::components::slider::QuickSlider;
use crate::widgets::components::toggle_tile::ToggleTile;
use crate::widgets::popup_base::PopupContainer;
use axis_domain::models::audio::AudioStatus;
use axis_domain::models::brightness::BrightnessStatus;
use axis_domain::models::popups::{PopupStatus, PopupType};
use axis_presentation::View;
use gtk4::{gio, glib};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

glib::wrapper! {
    pub struct QuickSettingsWindow(ObjectSubclass<imp::QuickSettingsWindow>)
        @extends gtk4::Widget, gtk4::Window, gtk4::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl QuickSettingsWindow {
    pub fn new(app: &libadwaita::Application) -> Self {
        glib::Object::builder().property("application", app).build()
    }
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct QuickSettingsWindow;

    #[glib::object_subclass]
    impl ObjectSubclass for QuickSettingsWindow {
        const NAME: &'static str = "QuickSettingsPopup";
        type Type = super::QuickSettingsWindow;
        type ParentType = gtk4::ApplicationWindow;
    }

    impl ObjectImpl for QuickSettingsWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.init_layer_shell();
            obj.set_layer(Layer::Top);
            obj.set_namespace(Some("quick-settings"));
            obj.set_anchor(Edge::Right, true);
            obj.set_anchor(Edge::Bottom, true);
            obj.set_margin(Edge::Bottom, 64);
            obj.set_margin(Edge::Right, 10);
            obj.set_default_size(320, -1);
            obj.set_keyboard_mode(KeyboardMode::OnDemand);
            obj.add_css_class("popup-window");
        }
    }

    impl WidgetImpl for QuickSettingsWindow {}
    impl WindowImpl for QuickSettingsWindow {}
    impl ApplicationWindowImpl for QuickSettingsWindow {}
}

#[derive(Clone)]
pub struct QuickSettingsPopup {
    window: QuickSettingsWindow,
    container: PopupContainer,
    grid: gtk4::Grid,
    volume_slider: QuickSlider,
    brightness_slider: QuickSlider,
    qs_stack: gtk4::Stack,
    battery_button: BatteryButton,
    power_actions: Rc<RefCell<Option<Rc<PowerActionStack>>>>,
    bottom_row: gtk4::Box,
    is_audio_updating: Rc<Cell<bool>>,
    is_bright_updating: Rc<Cell<bool>>,
    is_bright_dragging: Rc<Cell<bool>>,
    notification_presenter: Rc<RefCell<Option<Rc<NotificationPresenter>>>>,
    on_escape: FnCell0,
}

impl QuickSettingsPopup {
    pub fn new(app: &libadwaita::Application) -> Self {
        let window = QuickSettingsWindow::new(app);
        let container = PopupContainer::new();
        let grid = gtk4::Grid::new();

        let volume_slider = QuickSlider::new("audio-volume-high-symbolic");
        volume_slider.set_show_arrow(true);

        let brightness_slider = QuickSlider::new("display-brightness-symbolic");
        brightness_slider
            .scale()
            .set_adjustment(&gtk4::Adjustment::new(0.0, 0.0, 100.0, 1.0, 10.0, 0.0));

        let qs_stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::SlideLeftRight)
            .transition_duration(250)
            .vhomogeneous(false)
            .hhomogeneous(true)
            .interpolate_size(true)
            .build();

        let battery_button = BatteryButton::new();
        let bottom_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        let is_bright_dragging = Rc::new(Cell::new(false));

        let popup = Self {
            window,
            container,
            grid,
            volume_slider,
            brightness_slider,
            qs_stack,
            battery_button,
            power_actions: Rc::new(RefCell::new(None)),
            bottom_row,
            is_audio_updating: Rc::new(Cell::new(false)),
            is_bright_updating: Rc::new(Cell::new(false)),
            is_bright_dragging: is_bright_dragging.clone(),
            notification_presenter: Rc::new(RefCell::new(None)),
            on_escape: Rc::new(RefCell::new(None)),
        };

        popup.build_ui();
        popup.setup_key_controller();
        popup
    }

    fn build_ui(&self) {
        let main_page = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        main_page.add_css_class("quick-settings");

        self.grid.set_column_spacing(12);
        self.grid.set_row_spacing(12);
        self.grid.set_column_homogeneous(true);
        main_page.append(&self.grid);

        main_page.append(&self.volume_slider.container);

        let is_dragging = self.is_bright_dragging.clone();
        let gesture = gtk4::GestureClick::new();
        gesture.connect_pressed(move |_, _, _, _| {
            is_dragging.set(true);
        });
        let is_dragging_rel = self.is_bright_dragging.clone();
        gesture.connect_released(move |_, _, _, _| {
            is_dragging_rel.set(false);
        });
        self.brightness_slider.scale().add_controller(gesture);

        main_page.append(&self.brightness_slider.container);

        self.bottom_row.append(&self.battery_button.container);

        let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        self.bottom_row.append(&spacer);

        main_page.append(&self.bottom_row);

        self.qs_stack.add_named(&main_page, Some("main"));

        self.container.set_content(&self.qs_stack);
        self.window.set_child(Some(&self.container.container));
    }

    fn setup_key_controller(&self) {
        let stack = self.qs_stack.clone();
        let power_actions = self.power_actions.clone();
        let on_escape_c = self.on_escape.clone();
        let key_controller = gtk4::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            use gtk4::gdk::Key;
            if key == Key::Escape {
                if stack.visible_child_name().as_deref() != Some("main") {
                    stack.set_visible_child_name("main");
                    if let Some(pa) = power_actions.borrow().as_ref() {
                        pa.collapse_power_menu();
                    }
                    return gtk4::glib::Propagation::Stop;
                }
                if let Some(pa) = power_actions.borrow().as_ref()
                    && pa.is_power_expanded()
                {
                    pa.collapse_power_menu();
                    return gtk4::glib::Propagation::Stop;
                }
                if let Some(f) = on_escape_c.borrow().as_ref() {
                    f();
                }
                gtk4::glib::Propagation::Stop
            } else {
                gtk4::glib::Propagation::Proceed
            }
        });
        self.window.add_controller(key_controller);
    }

    pub fn setup_audio(&self, presenter: Rc<AudioPresenter>) {
        let view = Box::new(self.clone());
        presenter.add_view(view);
        self.on_volume_changed(Box::new(move |val| {
            presenter.handle_user_volume_change(val);
        }));
    }

    pub fn setup_brightness(&self, presenter: Rc<BrightnessPresenter>) {
        let view = Box::new(self.clone());
        presenter.add_view(view);
        self.on_brightness_changed(Box::new(move |val| {
            presenter.handle_user_change(val);
        }));
    }

    pub fn setup_battery(&self, presenter: Rc<BatteryPresenter>) {
        presenter.add_view(Box::new(self.battery_button.clone()));
    }

    pub fn setup_bottom_row(
        &self,
        battery_presenter: Rc<BatteryPresenter>,
        power_actions: Rc<PowerActionStack>,
    ) {
        self.setup_battery(battery_presenter);
        *self.power_actions.borrow_mut() = Some(power_actions.clone());
        self.append_power_actions(&power_actions.stack);
    }

    pub fn setup_toggle(
        &self,
        row: i32,
        col: i32,
        presenter: Rc<TogglePresenter>,
        arrow_target: Option<&str>,
    ) {
        let has_arrow = arrow_target.is_some();
        let tile = ToggleTile::new("", "image-missing-symbolic", has_arrow);
        if let Some(target) = arrow_target {
            let stack = self.qs_stack.clone();
            let target = target.to_string();
            tile.on_arrow_clicked(move || {
                stack.set_visible_child_name(&target);
            });
        }
        self.grid.attach(&tile.container, col, row, 1, 1);
        let view = Box::new(tile);
        glib::spawn_future_local(async move {
            presenter.bind(view).await;
        });
    }

    pub fn setup_notification_archive(&self, archive_container: gtk4::Revealer) {
        self.container.prepend_outside(&archive_container);
    }

    pub fn set_notification_presenter(&self, presenter: Rc<NotificationPresenter>) {
        *self.notification_presenter.borrow_mut() = Some(presenter);
    }

    pub fn on_escape(&self, f: Box<dyn Fn() + 'static>) {
        *self.on_escape.borrow_mut() = Some(f);
    }

    pub fn setup_wifi_sub_page(&self, presenter: Rc<NetworkPresenter>) {
        let stack = self.qs_stack.clone();
        let pres = presenter.clone();
        let scanned: Rc<Cell<bool>> = Rc::new(Cell::new(false));
        stack.connect_visible_child_name_notify(move |s| {
            if s.visible_child_name().as_deref() == Some("wifi") {
                if !scanned.get() {
                    pres.start_scan();
                    scanned.set(true);
                }
            } else {
                scanned.set(false);
            }
        });

        let page = Rc::new(crate::widgets::sub_pages::wifi_page::WifiPage::new(
            presenter,
            move || stack.set_visible_child_name("main"),
        ));
        self.qs_stack.add_named(&page.container, Some("wifi"));
    }

    pub fn setup_bluetooth_sub_page(&self, presenter: Rc<BluetoothPresenter>) {
        let stack = self.qs_stack.clone();
        let pres = presenter.clone();
        let scan_active: Rc<Cell<bool>> = Rc::new(Cell::new(false));
        stack.connect_visible_child_name_notify(move |s| {
            if s.visible_child_name().as_deref() == Some("bluetooth") {
                if !scan_active.get() {
                    pres.start_scan();
                    scan_active.set(true);
                }
            } else if scan_active.get() {
                pres.stop_scan();
                scan_active.set(false);
            }
        });

        let page =
            crate::widgets::sub_pages::bluetooth_page::BluetoothPage::new(presenter, move || {
                stack.set_visible_child_name("main")
            });
        self.qs_stack.add_named(&page.container, Some("bluetooth"));
    }

    pub fn setup_audio_sub_page(&self, presenter: Rc<AudioPresenter>) {
        let stack = self.qs_stack.clone();
        let page = crate::widgets::sub_pages::audio_page::AudioPage::new(presenter, move || {
            stack.set_visible_child_name("main")
        });
        let stack_nav = self.qs_stack.clone();
        self.volume_slider.on_arrow_clicked(move || {
            stack_nav.set_visible_child_name("audio");
        });
        self.qs_stack.add_named(&page.container, Some("audio"));
    }

    pub fn setup_nightlight_sub_page(&self, presenter: Rc<NightlightPresenter>) {
        let stack = self.qs_stack.clone();
        let page =
            crate::widgets::sub_pages::nightlight_page::NightlightPage::new(presenter, move || {
                stack.set_visible_child_name("main")
            });
        self.qs_stack.add_named(&page.container, Some("nightlight"));
    }

    pub fn reset_to_main(&self) {
        self.qs_stack.set_visible_child_name("main");
        if let Some(pa) = self.power_actions.borrow().as_ref() {
            pa.collapse_power_menu();
        }
    }

    fn append_power_actions(&self, power_actions: &gtk4::Stack) {
        self.bottom_row.append(power_actions);
    }

    fn on_volume_changed(&self, f: Box<dyn Fn(f64) + 'static>) {
        let slider = self.volume_slider.clone();
        let popup = self.clone();
        slider.scale().connect_value_changed(move |scale| {
            if !popup.is_audio_updating.get() {
                f(scale.value());
            }
        });
    }

    fn on_brightness_changed(&self, f: Box<dyn Fn(f64) + 'static>) {
        let slider = self.brightness_slider.clone();
        let popup = self.clone();
        slider.scale().connect_value_changed(move |scale| {
            if !popup.is_bright_updating.get() {
                f(scale.value());
            }
        });
    }
}

impl View<AudioStatus> for QuickSettingsPopup {
    fn render(&self, status: &AudioStatus) {
        let icon_name = audio_icon(status).to_string();
        let is_full = status.volume >= 0.99;
        self.volume_slider.set_icon(&icon_name);
        if is_full {
            self.volume_slider
                .scale()
                .remove_css_class("highlight-partial");
        } else {
            self.volume_slider
                .scale()
                .add_css_class("highlight-partial");
        }

        let scale = self.volume_slider.scale();
        if (scale.value() - status.volume).abs() > 0.01 {
            self.is_audio_updating.set(true);
            scale.set_value(status.volume);
            self.is_audio_updating.set(false);
        }
    }
}

impl View<BrightnessStatus> for QuickSettingsPopup {
    fn render(&self, status: &BrightnessStatus) {
        if !status.has_backlight {
            self.brightness_slider.container.set_visible(false);
            return;
        }

        self.brightness_slider.container.set_visible(true);
        self.brightness_slider
            .set_icon("display-brightness-symbolic");

        if !self.is_bright_dragging.get() {
            let scale = self.brightness_slider.scale();
            let slider_pct = status.percentage * 100.0;
            if (scale.value() - slider_pct).abs() > 1.0 {
                self.is_bright_updating.set(true);
                scale.set_value(slider_pct);
                self.is_bright_updating.set(false);
            }
        }
    }
}

impl View<PopupStatus> for QuickSettingsPopup {
    fn render(&self, status: &PopupStatus) {
        self.handle_status(status);
    }
}

impl PopupView for QuickSettingsPopup {
    fn get_type(&self) -> PopupType {
        PopupType::QuickSettings
    }
    fn popup_container(&self) -> PopupContainer {
        self.container.clone()
    }
    fn popup_window(&self) -> gtk4::ApplicationWindow {
        self.window.clone().upcast()
    }

    fn show(&self) {
        self.popup_container().animate_show(&self.popup_window());
        if let Some(presenter) = self.notification_presenter.borrow().as_ref() {
            presenter.set_popup_open(true);
        }
    }

    fn hide(&self) {
        self.popup_container().animate_hide(&self.popup_window());
        if let Some(presenter) = self.notification_presenter.borrow().as_ref() {
            presenter.set_popup_open(false);
        }
        self.reset_to_main();
    }
}
