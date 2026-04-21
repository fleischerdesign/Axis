use gtk4::gdk;
use gtk4::glib;
use gtk4::graphene;
use gtk4::gsk;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use std::cell::RefCell;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct BlurredPicture {
        pub(super) texture: RefCell<Option<gdk::Texture>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BlurredPicture {
        const NAME: &'static str = "AxisBlurredPicture";
        type Type = super::BlurredPicture;
        type ParentType = gtk4::Widget;
    }

    impl ObjectImpl for BlurredPicture {}

    impl WidgetImpl for BlurredPicture {
        fn snapshot(&self, snapshot: &gtk4::Snapshot) {
            let Some(ref texture) = *self.texture.borrow() else {
                return;
            };

            let w = self.obj().width() as f32;
            let h = self.obj().height() as f32;
            if w <= 0.0 || h <= 0.0 {
                return;
            }

            let tw = texture.width() as f32;
            let th = texture.height() as f32;

            let scale = (w / tw).max(h / th);
            let iw = tw * scale;
            let ih = th * scale;
            let x = (w - iw) / 2.0;
            let y = (h - ih) / 2.0;

            let bounds = graphene::Rect::new(x, y, iw, ih);
            let tex_node = gsk::TextureNode::new(texture, &bounds);
            let blur_node = gsk::BlurNode::new(tex_node, 30.0);
            snapshot.append_node(blur_node);
        }

        fn measure(&self, _orientation: gtk4::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            // Ein Hintergrund-Widget sollte keinen Platz anfordern (0,0),
            // aber bereit sein, unendlich viel Platz einzunehmen (-1,-1).
            (0, 0, -1, -1)
        }
    }
}

glib::wrapper! {
    pub struct BlurredPicture(ObjectSubclass<imp::BlurredPicture>)
        @extends gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl BlurredPicture {
    pub fn new(texture: &gdk::Texture) -> Self {
        let obj: Self = glib::Object::new();
        *obj.imp().texture.borrow_mut() = Some(texture.clone());
        obj
    }

    pub fn new_empty() -> Self {
        glib::Object::new()
    }

    pub fn set_texture(&self, texture: Option<gdk::Texture>) {
        *self.imp().texture.borrow_mut() = texture;
        self.queue_draw();
    }
}
