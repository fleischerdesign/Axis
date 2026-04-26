use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use libadwaita::prelude::*;
use gtk4::glib;
use axis_presentation::View;
use crate::presentation::tray::TrayView;
use crate::widgets::island::Island;
use axis_domain::models::tray::{TrayItemStatus, TrayStatus};

#[derive(Clone)]
pub struct TrayWidget {
    pub container: gtk4::Box,
    island: Island,
    inner_box: gtk4::Box,
    icons: Rc<RefCell<HashMap<String, gtk4::Image>>>,
    activate_cb: Rc<RefCell<Option<Rc<dyn Fn(String, i32, i32)>>>>,
    context_menu_cb: Rc<RefCell<Option<Rc<dyn Fn(String, i32, i32)>>>>,
    scroll_cb: Rc<RefCell<Option<Rc<dyn Fn(String, i32, String)>>>>,
}

impl TrayWidget {
    pub fn new() -> Self {
        let inner_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let island = Island::new();
        island.container.append(&inner_box);
        island.container.set_visible(false);

        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        container.append(&island.container);
        container.add_css_class("tray-widget");

        Self {
            container,
            island,
            inner_box,
            icons: Rc::new(RefCell::new(HashMap::new())),
            activate_cb: Rc::new(RefCell::new(None)),
            context_menu_cb: Rc::new(RefCell::new(None)),
            scroll_cb: Rc::new(RefCell::new(None)),
        }
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
        let inner_box = self.inner_box.clone();
        let island = self.island.clone();
        let icons = self.icons.clone();
        let activate_cb = self.activate_cb.clone();
        let context_menu_cb = self.context_menu_cb.clone();
        let scroll_cb = self.scroll_cb.clone();
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
                    inner_box.remove(&img);
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

                    inner_box.append(&img);
                    icons.borrow_mut().insert(item.bus_name.clone(), img);
                }
            }

            island.container.set_visible(!items.is_empty());

            glib::ControlFlow::Break
        });
    }
}

impl TrayView for TrayWidget {
    fn on_activate(&self, f: Box<dyn Fn(String, i32, i32) + 'static>) {
        *self.activate_cb.borrow_mut() = Some(Rc::new(f));
    }

    fn on_context_menu(&self, f: Box<dyn Fn(String, i32, i32) + 'static>) {
        *self.context_menu_cb.borrow_mut() = Some(Rc::new(f));
    }

    fn on_scroll(&self, f: Box<dyn Fn(String, i32, String) + 'static>) {
        *self.scroll_cb.borrow_mut() = Some(Rc::new(f));
    }
}
