use super::*;

impl Ui {
    pub(super) fn connect_focus(self: &Rc<Self>, app: &adw::Application) {
        let weak = self.weak();
        self.focus.connect_toggled(move |button| {
            if let Some(ui) = weak.upgrade() {
                ui.set_focus(button.is_active());
            }
        });
        for (icon, label, delta) in [
            ("go-previous-symbolic", "←", -1),
            ("go-next-symbolic", "→", 1),
            ("view-restore-symbolic", "Esc", 0),
        ] {
            let button = icon_button(icon, label);
            button.set_focus_on_click(false);
            let weak = self.weak();
            button.connect_clicked(move |_| {
                if let Some(ui) = weak.upgrade() {
                    if delta == 0 {
                        ui.set_focus(false);
                    } else {
                        ui.step(delta);
                    }
                }
            });
            self.focus_bar.append(&button);
        }
        let action = gio::SimpleAction::new("focus-reading", None);
        let weak = self.weak();
        action.connect_activate(move |_, _| {
            if let Some(ui) = weak.upgrade() {
                let active = !ui.state.borrow().focus_mode;
                ui.stack.set_visible_child_name("read");
                ui.set_focus(active);
            }
        });
        self.window.add_action(&action);
        app.set_accels_for_action("win.focus-reading", &["<Control><Shift>f"]);
        let keys = gtk::EventControllerKey::new();
        let weak = self.weak();
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape
                && let Some(ui) = weak.upgrade()
                && ui.focus_bar.is_visible()
                && !ui.note_popup.is_visible()
                && !ui.popup.is_visible()
            {
                ui.set_focus(false);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        self.window.add_controller(keys);
        let weak = self.weak();
        self.header_title.connect_title_visible_notify(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.apply_focus();
            }
        });
    }
    pub(super) fn set_focus(self: &Rc<Self>, active: bool) {
        let changed = self.state.borrow().focus_mode != active;
        self.state.borrow_mut().focus_mode = active;
        self.book_surface.cancel();
        self.apply_focus();
        self.schedule_book();
        if changed && self.ready.get() {
            self.core
                .post(json!({"op":"settings","key":"focusMode","value":active}));
        }
    }
    pub(super) fn apply_focus(&self) {
        let selected = self.state.borrow().focus_mode;
        let active = selected && self.stack.visible_child_name().as_deref() == Some("read");
        self.focus.set_active(selected);
        self.header.set_visible(!active);
        self.switcher
            .set_reveal(!active && self.header_title.is_title_visible());
        self.controls.set_visible(!active);
        self.footer.set_visible(!active);
        self.reference.set_visible(!active);
        self.subtitle.set_visible(!active);
        self.focus_bar.set_visible(active);
    }
    pub(super) fn relabel_focus(&self) {
        self.focus.set_tooltip_text(Some(
            &self.t("Concentrazione · Ctrl+Shift+F", "Focus · Ctrl+Shift+F"),
        ));
        for (index, (it, en)) in [
            ("Pagina precedente", "Previous page"),
            ("Pagina successiva", "Next page"),
            ("Esci dalla concentrazione · Esc", "Leave focus · Esc"),
        ]
        .into_iter()
        .enumerate()
        {
            if let Some(button) = self
                .focus_bar
                .observe_children()
                .item(index as u32)
                .and_downcast::<gtk::Button>()
            {
                button.set_tooltip_text(Some(&self.t(it, en)));
                button.update_property(&[gtk::accessible::Property::Label(&self.t(it, en))]);
            }
        }
    }
    pub(super) fn apply_reading_theme(&self) {
        let sepia = self.state.borrow().theme == "Sepia"
            && !adw::StyleManager::default().is_high_contrast();
        set_theme(&self.state.borrow().theme);
        if sepia {
            self.window.add_css_class("sepia");
            self.book_surface.add_css_class("sepia");
        } else {
            self.window.remove_css_class("sepia");
            self.book_surface.remove_css_class("sepia");
        }
        self.book_surface.cancel();
    }
    pub(super) fn apply_margins(&self) {
        let value = self.state.borrow().reading_margin;
        for view in [&self.reader, &self.reader_right] {
            view.set_left_margin(value);
            view.set_right_margin(value);
        }
        if let Some(views) = self.comparison_views.borrow_mut().as_mut() {
            views.stamp = None;
            for view in &views.readers {
                view.set_left_margin(value);
                view.set_right_margin(value);
            }
        }
    }
    pub(super) fn set_margins(self: &Rc<Self>, value: i32) {
        self.state.borrow_mut().reading_margin = value.clamp(16, 96);
        self.book_surface.cancel();
        self.apply_margins();
        self.schedule_book();
        self.schedule_comparison();
        if let Some(source) = self.margin_source.borrow_mut().take() {
            source.remove();
        }
        let weak = self.weak();
        *self.margin_source.borrow_mut() = Some(glib::timeout_add_local_once(
            Duration::from_millis(150),
            move || {
                if let Some(ui) = weak.upgrade() {
                    ui.margin_source.borrow_mut().take();
                    ui.core.post(json!({"op":"settings","key":"readingMargin","value":ui.state.borrow().reading_margin}));
                }
            },
        ));
    }
    pub(super) fn connect_note_indicator(self: &Rc<Self>, reader: &gtk::TextView, is_right: bool) {
        let click = gtk::GestureClick::new();
        click.set_button(1);
        click.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = self.weak();
        let view = reader.downgrade();
        click.connect_pressed(move |click, _, x, y| {
            if let (Some(ui), Some(view)) = (weak.upgrade(), view.upgrade()) {
                let (x, y) =
                    view.window_to_buffer_coords(gtk::TextWindowType::Widget, x as i32, y as i32);
                if let Some(iter) = view.iter_at_location(x, y) {
                    let index = {
                        let s = ui.state.borrow();
                        s.spans
                            .iter()
                            .enumerate()
                            .find(|(i, span)| {
                                iter.offset() == span.prefix + 1
                                    && !s.verses[*i].annotation.note.is_empty()
                            })
                            .map(|(i, _)| i)
                    };
                    if let Some(index) = index {
                        ui.active_right.set(is_right);
                        let mut s = ui.state.borrow_mut();
                        s.selection.clear();
                        s.position.verse = index;
                        drop(s);
                        view.buffer().place_cursor(&iter);
                        click.set_state(gtk::EventSequenceState::Claimed);
                        ui.show_note();
                    }
                }
            }
        });
        reader.add_controller(click);
    }
}
