use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use gtk4::{glib, gio};
use axis_domain::models::popups::PopupType;
use axis_domain::models::audio::AudioStatus;
use axis_domain::models::brightness::BrightnessStatus;
use crate::widgets::popup_base::PopupContainer;
use crate::widgets::components::slider::QuickSlider;
use crate::widgets::components::toggle_tile::ToggleTile;
use crate::widgets::components::battery_button::BatteryButton;
use crate::widgets::components::power_actions::PowerActionStack;
use crate::presentation::notifications::NotificationPresenter;
use axis_presentation::View;
use crate::presentation::popups::PopupView;
use crate::presentation::audio::{AudioPresenter, AudioView, audio_icon};
use crate::presentation::toggle::TogglePresenter;
use crate::presentation::brightness::{BrightnessPresenter, BrightnessView};
use crate::presentation::network::NetworkPresenter;
use crate::presentation::bluetooth::BluetoothPresenter;
use crate::presentation::nightlight::NightlightPresenter;
use crate::presentation::battery::BatteryPresenter;
use std::sync::Arc;
use std::rc::Rc;
use std::cell::{Cell, RefCell};

glib::wrapper! {
    pub struct QuickSettingsPopup(ObjectSubclass<imp::QuickSettingsPopup>)
        @extends gtk4::Widget, gtk4::Window, gtk4::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl QuickSettingsPopup {
    pub fn new(app: &libadwaita::Application) -> Self {
        glib::Object::builder()
            .property("application", app)
            .build()
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

    pub fn setup_battery(&self, presenter: Arc<BatteryPresenter>) {
        let battery_btn = self.imp().battery_button.get().expect("battery button not initialized").clone();
        presenter.add_view(Box::new(battery_btn));
    }

    pub fn setup_bottom_row(
        &self,
        battery_presenter: Arc<BatteryPresenter>,
        power_actions: Rc<PowerActionStack>,
    ) {
        self.setup_battery(battery_presenter);
        self.imp().power_actions.set(power_actions.clone()).expect("power actions already set");
        self.append_power_actions(&power_actions.stack);
    }

    pub fn setup_toggle<T: 'static>(&self, row: i32, col: i32, presenter: Rc<TogglePresenter<T>>, arrow_target: Option<&str>) {
        let has_arrow = arrow_target.is_some();
        let tile = ToggleTile::new("", "image-missing-symbolic", has_arrow);
        if let Some(target) = arrow_target {
            let stack = self.imp().qs_stack.get().expect("stack not initialized").clone();
            let target = target.to_string();
            tile.on_arrow_clicked(move || {
                stack.set_visible_child_name(&target);
            });
        }
        self.imp().grid.attach(&tile, col, row, 1, 1);
        let view = Box::new(tile);
        glib::spawn_future_local(async move { presenter.bind(view).await; });
    }

    pub fn setup_notification_archive(&self, archive_container: gtk4::Revealer) {
        self.imp().container.prepend_outside(&archive_container);
    }

    pub fn set_notification_presenter(&self, presenter: Rc<NotificationPresenter>) {
        *self.imp().notification_presenter.borrow_mut() = Some(presenter);
    }

    pub fn on_escape(&self, f: Box<dyn Fn() + 'static>) {
        *self.imp().on_escape.borrow_mut() = Some(f);
    }

    pub fn setup_wifi_sub_page(&self, presenter: Rc<NetworkPresenter>) {
        let stack = self.imp().qs_stack.get().expect("stack not initialized").clone();
        let page = Rc::new(crate::widgets::sub_pages::wifi_page::WifiPage::new(
            presenter,
            move || stack.set_visible_child_name("main"),
        ));
        self.imp().qs_stack.get().expect("stack not initialized").add_named(&page.container, Some("wifi"));
    }

    pub fn setup_bluetooth_sub_page(&self, presenter: Rc<BluetoothPresenter>) {
        let stack = self.imp().qs_stack.get().expect("stack not initialized").clone();
        let page = crate::widgets::sub_pages::bluetooth_page::BluetoothPage::new(
            presenter,
            move || stack.set_visible_child_name("main"),
        );
        self.imp().qs_stack.get().expect("stack not initialized").add_named(&page.container, Some("bluetooth"));
    }

    pub fn setup_audio_sub_page(&self, presenter: Rc<AudioPresenter>) {
        let stack = self.imp().qs_stack.get().expect("stack not initialized").clone();
        let page = crate::widgets::sub_pages::audio_page::AudioPage::new(
            presenter,
            move || stack.set_visible_child_name("main"),
        );
        if let Some(slider) = self.imp().volume_slider.get().cloned() {
            let stack_nav = self.imp().qs_stack.get().expect("stack not initialized").clone();
            slider.on_arrow_clicked(move || {
                stack_nav.set_visible_child_name("audio");
            });
        }
        self.imp().qs_stack.get().expect("stack not initialized").add_named(&page.container, Some("audio"));
    }

    pub fn setup_nightlight_sub_page(&self, presenter: Rc<NightlightPresenter>) {
        let stack = self.imp().qs_stack.get().expect("stack not initialized").clone();
        let page = crate::widgets::sub_pages::nightlight_page::NightlightPage::new(
            presenter,
            move || stack.set_visible_child_name("main"),
        );
        self.imp().qs_stack.get().expect("stack not initialized").add_named(&page.container, Some("nightlight"));
    }

    pub fn navigate_to(&self, page_name: &str) {
        self.imp().qs_stack.get().expect("stack not initialized").set_visible_child_name(page_name);
    }

    pub fn reset_to_main(&self) {
        self.imp().qs_stack.get().expect("stack not initialized").set_visible_child_name("main");
        if let Some(pa) = self.imp().power_actions.get() {
            pa.collapse_power_menu();
        }
    }

    fn setup_ui(&self) {
        let stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::SlideLeftRight)
            .transition_duration(250)
            .vhomogeneous(false)
            .hhomogeneous(true)
            .interpolate_size(true)
            .build();

        let main_page = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        main_page.add_css_class("quick-settings");

        let grid = self.imp().grid.clone();
        grid.set_column_spacing(12);
        grid.set_row_spacing(12);
        grid.set_column_homogeneous(true);
        main_page.append(&grid);

        let vol_slider = QuickSlider::new("audio-volume-high-symbolic");
        vol_slider.set_show_arrow(true);
        self.imp().volume_slider.set(vol_slider.clone()).expect("Failed to store vol slider");
        main_page.append(&vol_slider);

        let bright_slider = QuickSlider::new("display-brightness-symbolic");
        bright_slider.scale().set_adjustment(&gtk4::Adjustment::new(0.0, 0.0, 100.0, 1.0, 10.0, 0.0));

        let is_dragging = self.imp().is_bright_dragging.clone();
        let gesture = gtk4::GestureClick::new();
        gesture.connect_pressed(move |_, _, _, _| { is_dragging.set(true); });
        let is_dragging_rel = self.imp().is_bright_dragging.clone();
        gesture.connect_released(move |_, _, _, _| { is_dragging_rel.set(false); });
        bright_slider.scale().add_controller(gesture);

        self.imp().brightness_slider.set(bright_slider.clone()).expect("Failed to store bright slider");
        main_page.append(&bright_slider);

        let bottom_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);

        let battery_btn = BatteryButton::new();
        self.imp().battery_button.set(battery_btn.clone()).expect("battery button already set");
        bottom_row.append(&battery_btn);

        let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        bottom_row.append(&spacer);

        main_page.append(&bottom_row);

        stack.add_named(&main_page, Some("main"));

        self.imp().qs_stack.set(stack.clone()).expect("stack already set");
        self.imp().bottom_row.set(bottom_row).expect("bottom row already set");
        self.imp().container.set_content(&stack);
    }

    fn append_power_actions(&self, power_actions: &gtk4::Stack) {
        if let Some(bottom_row) = self.imp().bottom_row.get() {
            bottom_row.append(power_actions);
        }
    }
}

impl View<AudioStatus> for QuickSettingsPopup {
    fn render(&self, status: &AudioStatus) {
        if let Some(slider) = self.imp().volume_slider.get() {
            let icon_name = audio_icon(status).to_string();
            let is_full = status.volume >= 0.99;
            let slider_c = slider.clone();
            glib::idle_add_local(move || {
                slider_c.set_icon(&icon_name);
                if is_full { slider_c.scale().remove_css_class("highlight-partial"); }
                else { slider_c.scale().add_css_class("highlight-partial"); }
                glib::ControlFlow::Break
            });

            let scale = slider.scale();
            if (scale.value() - status.volume).abs() > 0.01 {
                self.imp().is_audio_updating.set(true);
                scale.set_value(status.volume);
                self.imp().is_audio_updating.set(false);
            }
        }
    }
}

impl AudioView for QuickSettingsPopup {
    fn on_volume_changed(&self, f: Box<dyn Fn(f64) + 'static>) {
        if let Some(slider) = self.imp().volume_slider.get() {
            let win = self.clone();
            slider.scale().connect_value_changed(move |scale| {
                if !win.imp().is_audio_updating.get() { f(scale.value()); }
            });
        }
    }

    fn on_set_default_sink(&self, _f: Box<dyn Fn(u32) + 'static>) {}
    fn on_set_default_source(&self, _f: Box<dyn Fn(u32) + 'static>) {}
    fn on_set_sink_input_volume(&self, _f: Box<dyn Fn(u32, f64) + 'static>) {}
}

impl View<BrightnessStatus> for QuickSettingsPopup {
    fn render(&self, status: &BrightnessStatus) {
        if let Some(slider) = self.imp().brightness_slider.get() {
            let icon_name = "display-brightness-symbolic".to_string();
            let slider_c = slider.clone();
            glib::idle_add_local(move || {
                slider_c.set_icon(&icon_name);
                glib::ControlFlow::Break
            });

            if !self.imp().is_bright_dragging.get() {
                let scale = slider.scale();
                if (scale.value() - status.percentage).abs() > 1.0 {
                    self.imp().is_bright_updating.set(true);
                    scale.set_value(status.percentage);
                    self.imp().is_bright_updating.set(false);
                }
            }
        }
    }
}

impl BrightnessView for QuickSettingsPopup {
    fn on_brightness_changed(&self, f: Box<dyn Fn(f64) + 'static>) {
        if let Some(slider) = self.imp().brightness_slider.get() {
            let win = self.clone();
            slider.scale().connect_value_changed(move |scale| {
                if !win.imp().is_bright_updating.get() { f(scale.value()); }
            });
        }
    }
}

impl PopupView for QuickSettingsPopup {
    fn get_type(&self) -> PopupType { PopupType::QuickSettings }
    fn popup_container(&self) -> PopupContainer { self.imp().container.clone() }
    fn popup_window(&self) -> gtk4::ApplicationWindow { self.clone().upcast() }

    fn show(&self) {
        self.popup_container().animate_show(&self.popup_window());
        if let Some(presenter) = self.imp().notification_presenter.borrow().as_ref() {
            presenter.set_popup_open(true);
        }
    }

    fn hide(&self) {
        self.popup_container().animate_hide(&self.popup_window());
        if let Some(presenter) = self.imp().notification_presenter.borrow().as_ref() {
            presenter.set_popup_open(false);
        }
        self.reset_to_main();
    }
}

mod imp {
    use super::*;
    use gtk4_layer_shell::{LayerShell, Layer, Edge, KeyboardMode};
    use std::cell::OnceCell;

    pub struct QuickSettingsPopup {
        pub container: PopupContainer,
        pub grid: gtk4::Grid,
        pub volume_slider: OnceCell<QuickSlider>,
        pub brightness_slider: OnceCell<QuickSlider>,
        pub qs_stack: OnceCell<gtk4::Stack>,
        pub battery_button: OnceCell<BatteryButton>,
        pub power_actions: OnceCell<Rc<PowerActionStack>>,
        pub bottom_row: OnceCell<gtk4::Box>,
        pub is_audio_updating: Cell<bool>,
        pub is_bright_updating: Cell<bool>,
        pub is_bright_dragging: Cell<bool>,
        pub notification_presenter: RefCell<Option<Rc<NotificationPresenter>>>,
        pub on_escape: Rc<RefCell<Option<Box<dyn Fn() + 'static>>>>,
    }

    impl Default for QuickSettingsPopup {
        fn default() -> Self {
            Self {
                container: PopupContainer::new(),
                grid: gtk4::Grid::new(),
                volume_slider: OnceCell::new(),
                brightness_slider: OnceCell::new(),
                qs_stack: OnceCell::new(),
                battery_button: OnceCell::new(),
                power_actions: OnceCell::new(),
                bottom_row: OnceCell::new(),
                is_audio_updating: Cell::new(false),
                is_bright_updating: Cell::new(false),
                is_bright_dragging: Cell::new(false),
                notification_presenter: RefCell::new(None),
                on_escape: Rc::new(RefCell::new(None)),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for QuickSettingsPopup {
        const NAME: &'static str = "QuickSettingsPopup";
        type Type = super::QuickSettingsPopup;
        type ParentType = gtk4::ApplicationWindow;
    }

    impl ObjectImpl for QuickSettingsPopup {
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
            obj.set_child(Some(&self.container));
            obj.setup_ui();

            let stack_c = self.qs_stack.clone();
            let power_actions_c = self.power_actions.clone();
            let on_escape_c = self.on_escape.clone();
            let key_controller = gtk4::EventControllerKey::new();
            key_controller.connect_key_pressed(move |_, key, _, _| {
                use gtk4::gdk::Key;
                if key == Key::Escape {
                    if let Some(stack) = stack_c.get() {
                        if stack.visible_child_name().as_deref() != Some("main") {
                            stack.set_visible_child_name("main");
                            if let Some(pa) = power_actions_c.get() {
                                pa.collapse_power_menu();
                            }
                            return gtk4::glib::Propagation::Stop;
                        }
                    }
                    if let Some(pa) = power_actions_c.get() {
                        if pa.is_power_expanded() {
                            pa.collapse_power_menu();
                            return gtk4::glib::Propagation::Stop;
                        }
                    }
                    if let Some(f) = on_escape_c.borrow().as_ref() {
                        f();
                    }
                    gtk4::glib::Propagation::Stop
                } else {
                    gtk4::glib::Propagation::Proceed
                }
            });
            obj.add_controller(key_controller);
        }
    }

    impl WidgetImpl for QuickSettingsPopup {}
    impl WindowImpl for QuickSettingsPopup {}
    impl ApplicationWindowImpl for QuickSettingsPopup {}
}
