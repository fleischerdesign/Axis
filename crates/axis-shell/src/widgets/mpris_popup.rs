use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use gtk4::{glib, gio};
use gtk4_layer_shell::{LayerShell, Layer, Edge, KeyboardMode};
use axis_domain::models::popups::{PopupType, PopupStatus};
use axis_domain::models::mpris::{MprisStatus, PlaybackState};
use crate::widgets::popup_base::PopupContainer;
use crate::presentation::popups::PopupView;
use axis_presentation::View;
use std::cell::RefCell;
use std::rc::Rc;

glib::wrapper! {
    pub struct MprisPopupWindow(ObjectSubclass<imp::MprisPopupWindow>)
        @extends gtk4::Widget, gtk4::Window, gtk4::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl MprisPopupWindow {
    pub fn new(app: &libadwaita::Application) -> Self {
        glib::Object::builder()
            .property("application", app)
            .build()
    }
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct MprisPopupWindow;

    #[glib::object_subclass]
    impl ObjectSubclass for MprisPopupWindow {
        const NAME: &'static str = "MprisPopup";
        type Type = super::MprisPopupWindow;
        type ParentType = gtk4::ApplicationWindow;
    }

    impl ObjectImpl for MprisPopupWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.init_layer_shell();
            obj.set_layer(Layer::Top);
            obj.set_namespace(Some("axis-mpris"));
            obj.set_anchor(Edge::Bottom, true);
            obj.set_margin(Edge::Bottom, 64);
            obj.set_default_size(320, -1);
            obj.set_keyboard_mode(KeyboardMode::OnDemand);
            obj.add_css_class("popup-window");
        }
    }

    impl WidgetImpl for MprisPopupWindow {}
    impl WindowImpl for MprisPopupWindow {}
    impl ApplicationWindowImpl for MprisPopupWindow {}
}

#[derive(Clone)]
pub struct MprisPopup {
    window: MprisPopupWindow,
    container: PopupContainer,
    art: gtk4::Picture,
    title_label: gtk4::Label,
    artist_label: gtk4::Label,
    album_label: gtk4::Label,
    prev_button: gtk4::Button,
    play_button: gtk4::Button,
    next_button: gtk4::Button,
    progress_bar: gtk4::ProgressBar,
    position_label: gtk4::Label,
    length_label: gtk4::Label,
    current_player_id: RefCell<Option<String>>,
    fallback_art: gtk4::Image,
    on_escape: Rc<RefCell<Option<Box<dyn Fn() + 'static>>>>,
    on_play_pause: Rc<RefCell<Option<Box<dyn Fn() + 'static>>>>,
    on_next: Rc<RefCell<Option<Box<dyn Fn() + 'static>>>>,
    on_previous: Rc<RefCell<Option<Box<dyn Fn() + 'static>>>>,
    on_visibility_change: Rc<RefCell<Option<Box<dyn Fn(bool) + 'static>>>>,
}

fn format_duration(microseconds: i64) -> String {
    let total_secs = microseconds.max(0) / 1_000_000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{mins}:{secs:02}")
}

impl MprisPopup {
    pub fn new(app: &libadwaita::Application) -> Self {
        let window = MprisPopupWindow::new(app);
        let container = PopupContainer::new();

        let on_escape: Rc<RefCell<Option<Box<dyn Fn() + 'static>>>> = Rc::new(RefCell::new(None));
        let on_play_pause: Rc<RefCell<Option<Box<dyn Fn() + 'static>>>> = Rc::new(RefCell::new(None));
        let on_next: Rc<RefCell<Option<Box<dyn Fn() + 'static>>>> = Rc::new(RefCell::new(None));
        let on_previous: Rc<RefCell<Option<Box<dyn Fn() + 'static>>>> = Rc::new(RefCell::new(None));
        let on_visibility_change: Rc<RefCell<Option<Box<dyn Fn(bool) + 'static>>>> = Rc::new(RefCell::new(None));

        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        content.add_css_class("mpris-popup");
        content.set_margin_top(16);
        content.set_margin_bottom(16);
        content.set_margin_start(16);
        content.set_margin_end(16);

        let art = gtk4::Picture::builder()
            .width_request(200)
            .height_request(200)
            .build();
        art.add_css_class("mpris-art");
        art.set_halign(gtk4::Align::Center);

        let fallback_art = gtk4::Image::from_icon_name("audio-x-generic-symbolic");
        fallback_art.set_pixel_size(200);
        art.set_paintable(fallback_art.paintable().as_ref());
        content.append(&art);

        let info_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        info_box.set_halign(gtk4::Align::Center);
        info_box.set_hexpand(true);

        let title_label = gtk4::Label::builder()
            .label("Not Playing")
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .max_width_chars(30)
            .css_classes(["title-3"])
            .build();
        title_label.set_halign(gtk4::Align::Center);

        let artist_label = gtk4::Label::builder()
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .max_width_chars(30)
            .build();
        artist_label.set_halign(gtk4::Align::Center);

        let album_label = gtk4::Label::builder()
            .css_classes(["dim-label"])
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .max_width_chars(30)
            .build();
        album_label.set_halign(gtk4::Align::Center);

        info_box.append(&title_label);
        info_box.append(&artist_label);
        info_box.append(&album_label);
        content.append(&info_box);

        let controls = gtk4::Box::new(gtk4::Orientation::Horizontal, 24);
        controls.set_halign(gtk4::Align::Center);
        controls.add_css_class("mpris-controls");

        let prev_button = gtk4::Button::builder()
            .icon_name("media-skip-backward-symbolic")
            .css_classes(["circular", "flat"])
            .build();

        let play_button = gtk4::Button::builder()
            .icon_name("media-playback-start-symbolic")
            .css_classes(["circular"])
            .build();

        let next_button = gtk4::Button::builder()
            .icon_name("media-skip-forward-symbolic")
            .css_classes(["circular", "flat"])
            .build();

        controls.append(&prev_button);
        controls.append(&play_button);
        controls.append(&next_button);
        content.append(&controls);

        let progress_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        progress_box.add_css_class("mpris-progress");

        let position_label = gtk4::Label::builder()
            .label("0:00")
            .css_classes(["caption", "dim-label"])
            .build();

        let progress_bar = gtk4::ProgressBar::builder()
            .hexpand(true)
            .valign(gtk4::Align::Center)
            .build();

        let length_label = gtk4::Label::builder()
            .label("0:00")
            .css_classes(["caption", "dim-label"])
            .build();

        progress_box.append(&position_label);
        progress_box.append(&progress_bar);
        progress_box.append(&length_label);
        content.append(&progress_box);

        container.set_content(&content);
        window.set_child(Some(&container.container));

        let esc_c = on_escape.clone();
        let key_controller = gtk4::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            use gtk4::gdk::Key;
            if key == Key::Escape {
                if let Some(f) = esc_c.borrow().as_ref() {
                    f();
                }
                gtk4::glib::Propagation::Stop
            } else {
                gtk4::glib::Propagation::Proceed
            }
        });
        window.add_controller(key_controller);

        let pp_c = on_play_pause.clone();
        play_button.connect_clicked(move |_| {
            if let Some(f) = pp_c.borrow().as_ref() {
                f();
            }
        });

        let nx_c = on_next.clone();
        next_button.connect_clicked(move |_| {
            if let Some(f) = nx_c.borrow().as_ref() {
                f();
            }
        });

        let pv_c = on_previous.clone();
        prev_button.connect_clicked(move |_| {
            if let Some(f) = pv_c.borrow().as_ref() {
                f();
            }
        });

        Self {
            window,
            container,
            art,
            title_label,
            artist_label,
            album_label,
            prev_button,
            play_button,
            next_button,
            progress_bar,
            position_label,
            length_label,
            current_player_id: RefCell::new(None),
            fallback_art,
            on_escape,
            on_play_pause,
            on_next,
            on_previous,
            on_visibility_change,
        }
    }

    pub fn on_escape(&self, f: Box<dyn Fn() + 'static>) {
        *self.on_escape.borrow_mut() = Some(f);
    }

    pub fn on_play_pause(&self, f: Box<dyn Fn() + 'static>) {
        *self.on_play_pause.borrow_mut() = Some(f);
    }

    pub fn on_next(&self, f: Box<dyn Fn() + 'static>) {
        *self.on_next.borrow_mut() = Some(f);
    }

    pub fn on_previous(&self, f: Box<dyn Fn() + 'static>) {
        *self.on_previous.borrow_mut() = Some(f);
    }

    pub fn on_visibility_change(&self, f: Box<dyn Fn(bool) + 'static>) {
        *self.on_visibility_change.borrow_mut() = Some(f);
    }

    pub fn update_position(&self, player_id: &str, position_us: i64, length_us: i64) {
        if Some(player_id) != self.current_player_id.borrow().as_deref() {
            return;
        }
        if length_us > 0 {
            let fraction = (position_us as f64 / length_us as f64).clamp(0.0, 1.0);
            self.progress_bar.set_fraction(fraction);
            self.position_label.set_label(&format_duration(position_us));
            self.length_label.set_label(&format_duration(length_us));
        }
    }
}

impl View<MprisStatus> for MprisPopup {
    fn render(&self, status: &MprisStatus) {
        *self.current_player_id.borrow_mut() = status.active_player_id.clone();
        match status.active_player() {
            Some(player) => {
                self.title_label.set_label(&player.title);
                self.artist_label.set_label(&player.artist);
                self.album_label.set_label(&player.album);

                let play_icon = match player.playback {
                    PlaybackState::Playing => "media-playback-pause-symbolic",
                    _ => "media-playback-start-symbolic",
                };
                self.play_button.set_icon_name(play_icon);

                self.prev_button.set_sensitive(player.can_go_previous);
                self.next_button.set_sensitive(player.can_go_next);
                self.play_button.set_sensitive(player.can_play || player.can_pause);

                if player.length_us > 0 {
                    let fraction = (player.position_us as f64 / player.length_us as f64).clamp(0.0, 1.0);
                    self.progress_bar.set_fraction(fraction);
                    self.position_label.set_label(&format_duration(player.position_us));
                    self.length_label.set_label(&format_duration(player.length_us));
                } else {
                    self.progress_bar.set_fraction(0.0);
                    self.position_label.set_label("0:00");
                    self.length_label.set_label("0:00");
                }

                if let Some(ref art_url) = player.art_url {
                    let file = gio::File::for_uri(art_url);
                    self.art.set_file(Some(&file));
                } else {
                    self.art.set_paintable(self.fallback_art.paintable().as_ref());
                }
            }
            None => {
                log::info!("[mpris-popup] No active player, showing placeholder");
                self.title_label.set_label("Not Playing");
                self.artist_label.set_label("");
                self.album_label.set_label("");
                self.play_button.set_icon_name("media-playback-start-symbolic");
                self.prev_button.set_sensitive(false);
                self.next_button.set_sensitive(false);
                self.play_button.set_sensitive(false);
                self.progress_bar.set_fraction(0.0);
                self.position_label.set_label("0:00");
                self.length_label.set_label("0:00");
                self.art.set_paintable(self.fallback_art.paintable().as_ref());
            }
        }
    }
}

impl View<PopupStatus> for MprisPopup {
    fn render(&self, status: &PopupStatus) {
        self.handle_status(status);
    }
}

impl PopupView for MprisPopup {
    fn get_type(&self) -> PopupType { PopupType::Mpris }
    fn popup_container(&self) -> PopupContainer { self.container.clone() }
    fn popup_window(&self) -> gtk4::ApplicationWindow { self.window.clone().upcast() }

    fn handle_status(&self, status: &PopupStatus) {
        let visible = status.active_popup == Some(self.get_type());
        if visible {
            self.show();
        } else {
            self.hide();
        }
        if let Some(cb) = self.on_visibility_change.borrow().as_ref() {
            cb(visible);
        }
    }
}
