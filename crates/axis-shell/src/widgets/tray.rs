use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use libadwaita::prelude::*;
use libadwaita::subclass::prelude::*;
use gtk4::glib;
use crate::presentation::presenter::View;
use crate::presentation::tray::TrayView;
use crate::widgets::island::Island;
use axis_domain::models::tray::{TrayItemStatus, TrayStatus};

glib::wrapper! {
    pub struct TrayWidget(ObjectSubclass<imp::TrayWidget>)
        @extends gtk4::Widget, gtk4::Box,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl TrayWidget {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn island(&self) -> &Island {
        &self.imp().island
    }

    fn pick_icon(item: &axis_domain::models::tray::TrayItem) -> &str {
        if item.status == TrayItemStatus::NeedsAttention && !item.attention_icon_name.is_empty() {
            &item.attention_icon_name
        } else if !item.icon_name.is_empty() {
            &item.icon_name
        } else {
            "application-x-executable-symbolic"
        }
    }
}

impl View<TrayStatus> for TrayWidget {
    fn render(&self, status: &TrayStatus) {
        let imp = self.imp();
        let container = imp.container.clone();
        let island = imp.island.clone();
        let icons = imp.icons.clone();
        let activate_cb = imp.activate_cb.clone();
        let context_menu_cb = imp.context_menu_cb.clone();
        let scroll_cb = imp.scroll_cb.clone();
        let items = status.items.clone();

        glib::idle_add_local(move || {
            let to_remove: Vec<String> = {
                let icons_ref = icons.borrow();
                let new_keys: Vec<&str> = items.iter().map(|i| i.bus_name.as_str()).collect();
                icons_ref
                    .keys()
                    .filter(|k| !new_keys.iter().any(|n| *n == k.as_str()))
                    .cloned()
                    .collect()
            };

            for key in to_remove {
                if let Some(img) = icons.borrow_mut().remove(&key) {
                    container.remove(&img);
                }
            }

            for item in &items {
                let exists = icons.borrow().contains_key(&item.bus_name);
                if exists {
                    let img = icons.borrow().get(&item.bus_name).cloned();
                    if let Some(img) = img {
                        let icon_name = Self::pick_icon(item).to_string();
                        img.set_icon_name(Some(&icon_name));
                        if !item.title.is_empty() {
                            img.set_tooltip_text(Some(&item.title));
                        }
                        img.set_visible(item.status != TrayItemStatus::Passive);
                    }
                } else {
                    let img = gtk4::Image::new();
                    img.set_pixel_size(16);
                    img.add_css_class("tray-icon");

                    let icon_name = Self::pick_icon(item).to_string();
                    img.set_icon_name(Some(&icon_name));

                    if !item.title.is_empty() {
                        img.set_tooltip_text(Some(&item.title));
                    }
                    img.set_visible(item.status != TrayItemStatus::Passive);

                    let click = gtk4::GestureClick::new();
                    click.set_button(0);

                    let bn_left = item.bus_name.clone();
                    let bn_right = item.bus_name.clone();
                    let bn_middle = item.bus_name.clone();
                    let act = activate_cb.clone();
                    let ctx = context_menu_cb.clone();

                    click.connect_pressed(move |gesture, _, x, y| {
                        match gesture.current_button() {
                            1 => {
                                if let Some(f) = act.borrow().as_ref() {
                                    f(bn_left.clone(), x as i32, y as i32);
                                }
                            }
                            3 => {
                                if let Some(f) = ctx.borrow().as_ref() {
                                    f(bn_right.clone(), x as i32, y as i32);
                                }
                            }
                            2 => {
                                if let Some(f) = act.borrow().as_ref() {
                                    f(bn_middle.clone(), x as i32, y as i32);
                                }
                            }
                            _ => {}
                        }
                    });

                    img.add_controller(click);

                    let scroll_controller = gtk4::EventControllerScroll::new(
                        gtk4::EventControllerScrollFlags::VERTICAL
                            | gtk4::EventControllerScrollFlags::HORIZONTAL,
                    );

                    let bn_scroll = item.bus_name.clone();
                    let scr = scroll_cb.clone();
                    scroll_controller.connect_scroll(move |_, dx, dy| {
                        let (delta, orientation) = if dx.abs() > dy.abs() {
                            (dx as i32, "horizontal".to_string())
                        } else {
                            (dy as i32, "vertical".to_string())
                        };
                        if let Some(f) = scr.borrow().as_ref() {
                            f(bn_scroll.clone(), delta, orientation);
                        }
                        glib::Propagation::Proceed
                    });

                    img.add_controller(scroll_controller);

                    container.append(&img);
                    icons.borrow_mut().insert(item.bus_name.clone(), img);
                }
            }

            island.set_visible(!items.is_empty());

            glib::ControlFlow::Break
        });
    }
}

impl TrayView for TrayWidget {
    fn on_activate(&self, f: Box<dyn Fn(String, i32, i32) + 'static>) {
        *self.imp().activate_cb.borrow_mut() = Some(Rc::new(f));
    }

    fn on_context_menu(&self, f: Box<dyn Fn(String, i32, i32) + 'static>) {
        *self.imp().context_menu_cb.borrow_mut() = Some(Rc::new(f));
    }

    fn on_scroll(&self, f: Box<dyn Fn(String, i32, String) + 'static>) {
        *self.imp().scroll_cb.borrow_mut() = Some(Rc::new(f));
    }
}

mod imp {
    use super::*;

    pub struct TrayWidget {
        pub island: Island,
        pub container: gtk4::Box,
        pub icons: RefCell<HashMap<String, gtk4::Image>>,
        pub activate_cb: RefCell<Option<Rc<dyn Fn(String, i32, i32)>>>,
        pub context_menu_cb: RefCell<Option<Rc<dyn Fn(String, i32, i32)>>>,
        pub scroll_cb: RefCell<Option<Rc<dyn Fn(String, i32, String)>>>,
    }

    impl Default for TrayWidget {
        fn default() -> Self {
            Self {
                island: Island::new(),
                container: gtk4::Box::new(gtk4::Orientation::Horizontal, 0),
                icons: RefCell::new(HashMap::new()),
                activate_cb: RefCell::new(None),
                context_menu_cb: RefCell::new(None),
                scroll_cb: RefCell::new(None),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TrayWidget {
        const NAME: &'static str = "TrayWidget";
        type Type = super::TrayWidget;
        type ParentType = gtk4::Box;
    }

    impl ObjectImpl for TrayWidget {
        fn constructed(&self) {
            self.parent_constructed();
            self.container.set_spacing(8);
            self.island.append(&self.container);
            self.obj().append(&self.island);
            self.island.set_visible(false);
            self.obj().add_css_class("tray-widget");
        }
    }

    impl WidgetImpl for TrayWidget {}
    impl BoxImpl for TrayWidget {}
}
