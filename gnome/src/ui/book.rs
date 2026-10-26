use super::book_math;
use adw::prelude::*;
use gtk::{gdk, glib, graphene, gsk, subclass::prelude::*};
use std::cell::{Cell, OnceCell, RefCell};

mod imp {
    use super::*;
    #[derive(Default)]
    pub struct BookSurface {
        pub child: OnceCell<gtk::Widget>,
        pub texture: RefCell<Option<gdk::Texture>>,
        pub tick: RefCell<Option<gtk::TickCallbackId>>,
        pub progress: Cell<f64>,
        pub forward: Cell<bool>,
        pub started: Cell<i64>,
        pub turns: Cell<u32>,
        pub allocated: Cell<(i32, i32)>,
        pub generation: Cell<u64>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for BookSurface {
        const NAME: &'static str = "BibbiaBookSurface";
        type Type = super::BookSurface;
        type ParentType = gtk::Widget;
    }
    impl ObjectImpl for BookSurface {
        fn dispose(&self) {
            self.obj().cancel();
            if let Some(child) = self.child.get() {
                child.unparent();
            }
        }
    }
    impl WidgetImpl for BookSurface {
        fn measure(&self, orientation: gtk::Orientation, size: i32) -> (i32, i32, i32, i32) {
            self.child
                .get()
                .map(|c| c.measure(orientation, size))
                .unwrap_or((0, 0, -1, -1))
        }
        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            if self.allocated.replace((width, height)) != (width, height) {
                self.obj().cancel();
            }
            if let Some(child) = self.child.get() {
                child.allocate(width, height, baseline, None);
            }
        }
        fn unmap(&self) {
            self.obj().cancel();
            self.parent_unmap();
        }
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let obj = self.obj();
            if let Some(child) = self.child.get() {
                obj.snapshot_child(child, snapshot);
            }
            let texture = self.texture.borrow();
            let Some(texture) = texture.as_ref() else {
                return;
            };
            let (width, height) = (obj.width() as f32, obj.height() as f32);
            if width < 1.0 || height < 1.0 {
                return;
            }
            let paper = if adw::StyleManager::default().is_dark() {
                gdk::RGBA::new(0.141, 0.137, 0.125, 1.0)
            } else {
                gdk::RGBA::new(1.0, 0.992, 0.969, 1.0)
            };
            let full = graphene::Rect::new(0.0, 0.0, width, height);
            snapshot.push_clip(&full);
            let forward = self.forward.get();
            let shadow_width = width * 0.36;
            let shadow_x = if forward { 0.0 } else { width - shadow_width };
            let opacity = (0.18 * (std::f64::consts::PI * self.progress.get()).sin()) as f32;
            snapshot.append_linear_gradient(
                &graphene::Rect::new(shadow_x, 0.0, shadow_width, height),
                &graphene::Point::new(if forward { shadow_x } else { width }, 0.0),
                &graphene::Point::new(if forward { shadow_width } else { shadow_x }, 0.0),
                &[
                    gsk::ColorStop::new(0.0, gdk::RGBA::new(0.0, 0.0, 0.0, opacity)),
                    gsk::ColorStop::new(1.0, gdk::RGBA::new(0.0, 0.0, 0.0, 0.0)),
                ],
            );
            // One cached texture, sampled through narrow affine projections.
            // This path works with GSK GL/Vulkan and the Cairo renderer.
            for strip in book_math::frame(
                width as f64,
                height as f64,
                self.progress.get(),
                self.forward.get(),
            ) {
                if strip.scale_x.abs() < 0.0001 {
                    continue;
                }
                snapshot.save();
                snapshot.translate(&graphene::Point::new(strip.x, strip.y));
                snapshot.scale(strip.scale_x, strip.scale_y);
                let clip =
                    graphene::Rect::new(0.0, 0.0, width / book_math::STRIPS as f32 + 0.7, height);
                snapshot.push_clip(&clip);
                snapshot.append_color(&paper, &clip);
                if strip.front {
                    snapshot.append_texture(
                        texture,
                        &graphene::Rect::new(-strip.source, 0.0, width, height),
                    );
                }
                snapshot.append_color(&gdk::RGBA::new(0.0, 0.0, 0.0, strip.shade), &clip);
                snapshot.pop();
                snapshot.restore();
            }
            snapshot.pop();
        }
    }
}

glib::wrapper! {
    pub struct BookSurface(ObjectSubclass<imp::BookSurface>)
        @extends gtk::Widget, @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl BookSurface {
    pub fn new(child: &impl IsA<gtk::Widget>) -> Self {
        let obj: Self = glib::Object::new();
        let child = child.clone().upcast::<gtk::Widget>();
        child.set_parent(&obj);
        let _ = obj.imp().child.set(child);
        obj.set_hexpand(true);
        obj.set_vexpand(true);
        obj.set_overflow(gtk::Overflow::Hidden);
        let weak = obj.downgrade();
        if let Some(settings) = gtk::Settings::default() {
            settings.connect_gtk_enable_animations_notify(move |settings| {
                if !settings.is_gtk_enable_animations()
                    && let Some(obj) = weak.upgrade()
                {
                    obj.cancel();
                }
            });
        }
        let weak = obj.downgrade();
        adw::StyleManager::default().connect_dark_notify(move |_| {
            if let Some(obj) = weak.upgrade() {
                obj.cancel();
            }
        });
        let click = gtk::EventControllerLegacy::new();
        click.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = obj.downgrade();
        click.connect_event(move |_, event| {
            if matches!(
                event.event_type(),
                gdk::EventType::ButtonPress
                    | gdk::EventType::TouchBegin
                    | gdk::EventType::Scroll
                    | gdk::EventType::KeyPress
            ) && let Some(obj) = weak.upgrade()
            {
                obj.cancel();
            }
            glib::Propagation::Proceed
        });
        obj.add_controller(click);
        obj
    }

    pub fn capture(&self) -> Option<gdk::Texture> {
        self.cancel();
        if !self.is_mapped() || !Self::motion_allowed() || self.width() <= 20 || self.height() <= 20
        {
            return None;
        }
        let child = self.imp().child.get()?;
        let renderer = self.native()?.renderer()?;
        let width = self.width() as f64;
        let height = self.height() as f64;
        let scale = (self.scale_factor() as f64)
            .min(1600.0 / width)
            .min((2_000_000.0 / (width * height)).sqrt());
        let paintable = gtk::WidgetPaintable::new(Some(child));
        let snapshot = gtk::Snapshot::new();
        snapshot.scale(scale as f32, scale as f32);
        paintable.snapshot(&snapshot, width, height);
        let node = snapshot.to_node()?;
        Some(renderer.render_texture(
            &node,
            Some(&graphene::Rect::new(
                0.0,
                0.0,
                (width * scale) as f32,
                (height * scale) as f32,
            )),
        ))
    }

    pub fn motion_allowed() -> bool {
        gtk::Settings::default().is_some_and(|s| s.is_gtk_enable_animations())
    }

    pub fn turn(&self, texture: Option<gdk::Texture>, forward: bool) {
        self.cancel();
        let Some(texture) = texture else { return };
        if !Self::motion_allowed() || !self.is_mapped() {
            return;
        }
        let imp = self.imp();
        imp.texture.replace(Some(texture));
        imp.progress.set(0.0);
        imp.forward.set(forward);
        imp.started.set(0);
        imp.turns.set(imp.turns.get() + 1);
        self.queue_draw();
        let id = self.add_tick_callback(move |obj, clock| {
            let imp = obj.imp();
            if imp.started.get() == 0 {
                imp.started.set(clock.frame_time());
            }
            let progress = (clock.frame_time() - imp.started.get()) as f64 / book_math::DURATION_US;
            imp.progress.set(progress.min(1.0));
            obj.queue_draw();
            if progress >= 1.0 || !Self::motion_allowed() {
                imp.texture.borrow_mut().take();
                imp.tick.borrow_mut().take();
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
        imp.tick.replace(Some(id));
    }

    pub fn cancel(&self) {
        let imp = self.imp();
        imp.generation.set(imp.generation.get() + 1);
        if let Some(tick) = imp.tick.borrow_mut().take() {
            tick.remove();
        }
        if imp.texture.borrow_mut().take().is_some() {
            self.queue_draw();
        }
    }

    pub fn is_turning(&self) -> bool {
        self.imp().texture.borrow().is_some()
    }
    pub fn turns_started(&self) -> u32 {
        self.imp().turns.get()
    }
    pub fn generation(&self) -> u64 {
        self.imp().generation.get()
    }
}
