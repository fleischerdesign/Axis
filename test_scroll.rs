use gtk4::prelude::*;
fn test() {
    let scroll = gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::HORIZONTAL);
    scroll.connect_scroll_begin(|_| {});
    scroll.connect_scroll_end(|_| {});
}
