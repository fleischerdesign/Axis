use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use gtk4::{glib, gio};
use gtk4_layer_shell::{LayerShell, Layer, Edge, KeyboardMode};
use axis_domain::models::launcher::LauncherItem;
use axis_domain::models::launcher::LauncherStatus;
use axis_domain::models::popups::PopupType;
use axis_domain::models::popups::PopupStatus;
use crate::widgets::popup_base::PopupContainer;
use crate::widgets::components::list_row::ListRow;
use crate::presentation::popups::PopupView;
use axis_presentation::View;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::Arc;
use std::rc::Rc;

glib::wrapper! {
    pub struct LauncherPopupWindow(ObjectSubclass<imp::LauncherPopupWindow>)
        @extends gtk4::Widget, gtk4::Window, gtk4::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl LauncherPopupWindow {
    pub fn new(app: &libadwaita::Application) -> Self {
        glib::Object::builder()
            .property("application", app)
            .build()
    }
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct LauncherPopupWindow;

    #[glib::object_subclass]
    impl ObjectSubclass for LauncherPopupWindow {
        const NAME: &'static str = "AxisLauncherPopup";
        type Type = super::LauncherPopupWindow;
        type ParentType = gtk4::ApplicationWindow;
    }

    impl ObjectImpl for LauncherPopupWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.init_layer_shell();
            obj.set_layer(Layer::Top);
            obj.set_namespace(Some("axis-launcher"));
            obj.set_anchor(Edge::Left, true);
            obj.set_anchor(Edge::Bottom, true);
            obj.set_margin(Edge::Bottom, 64);
            obj.set_margin(Edge::Left, 10);
            obj.set_default_size(380, -1);
            obj.set_keyboard_mode(KeyboardMode::Exclusive);
            obj.add_css_class("popup-window");
        }
    }

    impl WidgetImpl for LauncherPopupWindow {}
    impl WindowImpl for LauncherPopupWindow {}
    impl ApplicationWindowImpl for LauncherPopupWindow {}
}

#[derive(Clone)]
pub struct RowEntry {
    row: ListRow,
    list_box_row: gtk4::ListBoxRow,
}

pub struct Callbacks {
    on_search: RefCell<Option<Box<dyn Fn(&str) + 'static>>>,
    on_select_next: RefCell<Option<Box<dyn Fn() + 'static>>>,
    on_select_prev: RefCell<Option<Box<dyn Fn() + 'static>>>,
    on_activate: RefCell<Option<Box<dyn Fn(Option<usize>) + 'static>>>,
    on_escape: RefCell<Option<Box<dyn Fn() + 'static>>>,
}

impl Callbacks {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            on_search: RefCell::new(None),
            on_select_next: RefCell::new(None),
            on_select_prev: RefCell::new(None),
            on_activate: RefCell::new(None),
            on_escape: RefCell::new(None),
        })
    }
}

#[derive(Clone)]
pub struct LauncherPopup {
    window: LauncherPopupWindow,
    container: PopupContainer,
    entry: gtk4::Entry,
    list: gtk4::ListBox,
    scrolled: gtk4::ScrolledWindow,
    detail_title: gtk4::Label,
    detail_desc: gtk4::Label,
    detail_revealer: gtk4::Revealer,
    rows: Rc<RefCell<HashMap<String, RowEntry>>>,
    callbacks: Arc<Callbacks>,
}

impl LauncherPopup {
    pub fn new(app: &libadwaita::Application) -> Self {
        let window = LauncherPopupWindow::new(app);

        let container = PopupContainer::new();

        let entry = gtk4::Entry::builder()
            .placeholder_text("Search, Find, Do...")
            .hexpand(true)
            .css_classes(vec!["qs-entry"])
            .build();

        let list = gtk4::ListBox::builder()
            .css_classes(vec!["qs-list", "launcher-list"])
            .selection_mode(gtk4::SelectionMode::Single)
            .build();

        let scrolled = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .min_content_height(400)
            .max_content_height(400)
            .build();
        scrolled.add_css_class("qs-scrolled");

        let detail_revealer = gtk4::Revealer::builder()
            .transition_type(gtk4::RevealerTransitionType::SlideRight)
            .transition_duration(250)
            .build();

        let detail_title = gtk4::Label::builder()
            .halign(gtk4::Align::Start)
            .css_classes(vec!["subpage-title"])
            .wrap(true)
            .build();

        let detail_desc = gtk4::Label::builder()
            .halign(gtk4::Align::Start)
            .wrap(true)
            .css_classes(vec!["list-sublabel"])
            .build();

        let popup = Self {
            window,
            container,
            entry,
            list,
            scrolled,
            detail_title,
            detail_desc,
            detail_revealer,
            rows: Rc::new(RefCell::new(HashMap::new())),
            callbacks: Callbacks::new(),
        };

        popup.build_ui();
        popup
    }

    fn build_ui(&self) {
        let content = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        content.add_css_class("qs-panel");
        content.set_width_request(380);

        let left_pane = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        left_pane.set_width_request(380);
        left_pane.set_hexpand(true);

        let entry_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        entry_box.set_margin_bottom(12);
        entry_box.append(&self.entry);
        left_pane.append(&entry_box);

        self.scrolled.set_child(Some(&self.list));
        left_pane.append(&self.scrolled);

        content.append(&left_pane);

        let detail_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        detail_box.set_width_request(280);
        detail_box.set_margin_start(16);
        detail_box.set_margin_end(16);
        detail_box.add_css_class("launcher-details");
        detail_box.append(&self.detail_title);
        detail_box.append(&self.detail_desc);
        self.detail_revealer.set_child(Some(&detail_box));
        content.append(&self.detail_revealer);

        self.container.set_content(&content);
        self.window.set_child(Some(&self.container.container));

        let cbs = self.callbacks.clone();
        let list = self.list.clone();
        list.connect_row_activated(move |_, row| {
            let idx = row.index() as usize;
            if let Some(f) = cbs.on_activate.borrow().as_ref() {
                f(Some(idx));
            }
        });

        let cbs = self.callbacks.clone();
        let entry = self.entry.clone();
        entry.connect_changed(move |e| {
            if let Some(f) = cbs.on_search.borrow().as_ref() {
                f(&e.text());
            }
        });

        let cbs = self.callbacks.clone();
        let key_controller = gtk4::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, state| {
            use gtk4::gdk::{Key, ModifierType};
            match key {
                Key::Escape => {
                    if let Some(f) = cbs.on_escape.borrow().as_ref() { f(); }
                    gtk4::glib::Propagation::Stop
                }
                Key::Down => {
                    if let Some(f) = cbs.on_select_next.borrow().as_ref() { f(); }
                    gtk4::glib::Propagation::Stop
                }
                Key::Up => {
                    if let Some(f) = cbs.on_select_prev.borrow().as_ref() { f(); }
                    gtk4::glib::Propagation::Stop
                }
                Key::Tab => {
                    if state.contains(ModifierType::SHIFT_MASK) {
                        if let Some(f) = cbs.on_select_prev.borrow().as_ref() { f(); }
                    } else {
                        if let Some(f) = cbs.on_select_next.borrow().as_ref() { f(); }
                    }
                    gtk4::glib::Propagation::Stop
                }
                _ => gtk4::glib::Propagation::Proceed,
            }
        });
        self.entry.add_controller(key_controller);

        let cbs = self.callbacks.clone();
        self.entry.connect_activate(move |_| {
            if let Some(f) = cbs.on_activate.borrow().as_ref() {
                f(None);
            }
        });
    }

    pub fn on_search(&self, f: Box<dyn Fn(&str) + 'static>) {
        *self.callbacks.on_search.borrow_mut() = Some(f);
    }

    pub fn on_select_next(&self, f: Box<dyn Fn() + 'static>) {
        *self.callbacks.on_select_next.borrow_mut() = Some(f);
    }

    pub fn on_select_prev(&self, f: Box<dyn Fn() + 'static>) {
        *self.callbacks.on_select_prev.borrow_mut() = Some(f);
    }

    pub fn on_activate(&self, f: Box<dyn Fn(Option<usize>) + 'static>) {
        *self.callbacks.on_activate.borrow_mut() = Some(f);
    }

    pub fn on_escape(&self, f: Box<dyn Fn() + 'static>) {
        *self.callbacks.on_escape.borrow_mut() = Some(f);
    }

    pub fn update_results(&self, results: &[LauncherItem], selected_index: Option<usize>) {
        let mut rows = self.rows.borrow_mut();

        let ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
        crate::utils::reconcile::reconcile(&mut rows, &ids, |_, entry| {
            self.list.remove(&entry.list_box_row);
        });

        for (idx, item) in results.iter().enumerate() {
            if let Some(entry) = rows.get(&item.id) {
                entry.row.set_title(&item.title);
                entry.row.set_icon(&item.icon_name);
                entry.row.set_subtitle(item.description.as_deref());
                continue;
            }

            let row = ListRow::new(&item.title, &item.icon_name);
            row.set_subtitle(item.description.as_deref());
            let list_box_row = gtk4::ListBoxRow::builder()
                .selectable(false)
                .activatable(true)
                .child(&row.container)
                .build();

            rows.insert(
                item.id.clone(),
                RowEntry { row, list_box_row: list_box_row.clone() },
            );
            self.list.insert(&list_box_row, idx as i32);
        }

        if results.is_empty() {
            self.list.unselect_all();
            self.detail_revealer.set_reveal_child(false);
            self.container.set_width_request(380);
            for entry in rows.values() {
                entry.row.container.remove_css_class("launcher-selected");
            }
        } else if let Some(idx) = selected_index {
            for entry in rows.values() {
                entry.row.container.remove_css_class("launcher-selected");
            }

            if let Some(row) = self.list.row_at_index(idx as i32) {
                self.list.select_row(Some(&row));

                for entry in rows.values() {
                    if entry.list_box_row.index() == row.index() {
                        entry.row.container.add_css_class("launcher-selected");
                        break;
                    }
                }

                let adj = self.scrolled.vadjustment();
                if let Some(point) = row.compute_point(&self.list, &gtk4::graphene::Point::new(0.0, 0.0)) {
                    let row_y = point.y() as f64;
                    let row_h = row.height() as f64;
                    let page = adj.page_size();
                    let target = (row_y - (page / 2.0) + (row_h / 2.0)).clamp(0.0, adj.upper() - page);
                    smooth_scroll(&adj, target);
                }

                if let Some(item) = results.get(idx) {
                    self.detail_title.set_text(&item.title);
                    self.detail_desc.set_text(item.description.as_deref().unwrap_or(""));
                    self.detail_revealer.set_reveal_child(true);
                    self.container.set_width_request(680);
                }
            }
        }
    }

    pub fn clear_and_focus(&self) {
        self.entry.set_text("");
        self.entry.grab_focus();

        if let Some(f) = self.callbacks.on_search.borrow().as_ref() {
            f("");
        }
    }
}

fn smooth_scroll(adj: &gtk4::Adjustment, target: f64) {
    const STEPS: u32 = 8;
    const INTERVAL: std::time::Duration = std::time::Duration::from_millis(15);

    let start = adj.value();
    let delta = target - start;
    if delta.abs() < 1.0 { return; }

    let adj = adj.clone();
    let step = Rc::new(Cell::new(0u32));

    glib::timeout_add_local(INTERVAL, move || {
        let s = step.get() + 1;
        step.set(s);

        let t = s as f64 / STEPS as f64;
        let ease = 1.0 - (1.0 - t).powi(3);
        adj.set_value(start + delta * ease);

        if s >= STEPS {
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });
}

impl View<PopupStatus> for LauncherPopup {
    fn render(&self, status: &PopupStatus) {
        self.handle_status(status);
    }
}

impl PopupView for LauncherPopup {
    fn get_type(&self) -> PopupType { PopupType::Launcher }
    fn popup_container(&self) -> PopupContainer { self.container.clone() }
    fn popup_window(&self) -> gtk4::ApplicationWindow { self.window.clone().upcast() }

    fn show(&self) {
        self.popup_container().animate_show(&self.popup_window());
        self.clear_and_focus();
    }
}

impl View<LauncherStatus> for LauncherPopup {
    fn render(&self, status: &LauncherStatus) {
        self.update_results(&status.results, status.selected_index);
    }
}
