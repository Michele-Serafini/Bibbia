use super::*;
use bible_core::Annotation;

impl Ui {
    pub(super) fn connect_annotations(self: &Rc<Self>, verse_menu: &gtk::MenuButton) {
        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        margin(&toolbar, 6);
        toolbar.append(&self.underline);
        let palette = gtk::MenuButton::builder()
            .icon_name("color-select-symbolic")
            .tooltip_text("Evidenzia / Highlight")
            .build();
        let colors = gtk::Popover::new();
        let box_ = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        margin(&box_, 8);
        for (key, label, css) in [
            ("yellow", "Giallo / Yellow", "highlight-yellow"),
            ("green", "Verde / Green", "highlight-green"),
            ("blue", "Blu / Blue", "highlight-blue"),
            ("pink", "Rosa / Pink", "highlight-pink"),
        ] {
            let button = gtk::Button::builder()
                .label("●")
                .tooltip_text(label)
                .css_classes([css, "circular"])
                .build();
            let w = self.weak();
            let pop = colors.clone();
            button.connect_clicked(move |_| {
                pop.popdown();
                if let Some(ui) = w.upgrade() {
                    let u = ui.clone();
                    ui.launch(async move { u.mark(key).await });
                }
            });
            box_.append(&button);
        }
        colors.set_child(Some(&box_));
        palette.set_popover(Some(&colors));
        toolbar.append(&palette);
        for (name, icon, tooltip) in [
            ("bookmark", "bookmark-new-symbolic", "Segnalibro / Bookmark"),
            ("note", "document-edit-symbolic", "Nota / Note"),
            ("copy", "edit-copy-symbolic", "Copia / Copy"),
            (
                "clear",
                "edit-clear-symbolic",
                "Rimuovi formattazione / Clear formatting",
            ),
        ] {
            let button = icon_button(icon, tooltip);
            toolbar.append(&button);
            let w = self.weak();
            button.connect_clicked(move |_| {
                if let Some(ui) = w.upgrade() {
                    ui.annotation_action(name);
                }
            });
        }
        self.popup.set_child(Some(&toolbar));
        let w = self.weak();
        self.underline.connect_clicked(move |_| {
            if let Some(ui) = w.upgrade() {
                let u = ui.clone();
                ui.launch(async move { u.mark("underline").await });
            }
        });
        let menu = gio::Menu::new();
        for (name, it, en) in [
            ("underline", "Sottolinea", "Underline"),
            ("bookmark", "Segnalibro", "Bookmark"),
            ("note", "Nota", "Note"),
            ("clear", "Rimuovi formattazione", "Clear formatting"),
        ] {
            menu.append(Some(&format!("{it} / {en}")), Some(&format!("win.{name}")));
            let action = gio::SimpleAction::new(name, None);
            let w = self.weak();
            action.connect_activate(move |_, _| {
                if let Some(ui) = w.upgrade() {
                    ui.capture_selection();
                    ui.annotation_action(name);
                }
            });
            self.window.add_action(&action);
        }
        self.reader.set_extra_menu(Some(&menu));
        self.reader_right.set_extra_menu(Some(&menu));
        verse_menu.set_menu_model(Some(&menu));
        for (reader, is_right) in [(&self.reader, false), (&self.reader_right, true)] {
            self.connect_note_indicator(reader, is_right);
            let events = gtk::EventControllerLegacy::new();
            events.set_propagation_phase(gtk::PropagationPhase::Capture);
            let w = self.weak();
            events.connect_event(move |_, event| {
                if let Some(button) = event.downcast_ref::<gdk::ButtonEvent>()
                    && button.button() != 1
                {
                    if let Some(ui) = w.upgrade() {
                        ui.context_click.set(true);
                        ui.cancel_selection();
                        ui.popup.popdown();
                    }
                    return glib::Propagation::Proceed;
                }
                if let Some(ui) = w.upgrade() {
                    match event.event_type() {
                        gdk::EventType::KeyPress => ui.context_click.set(false),
                        gdk::EventType::ButtonPress => {
                            ui.active_right.set(is_right);
                            ui.context_click.set(false);
                            ui.mouse_down.set(true);
                            ui.cancel_selection();
                        }
                        gdk::EventType::ButtonRelease | gdk::EventType::TouchEnd => {
                            ui.mouse_down.set(false);
                            let weak = ui.weak();
                            glib::idle_add_local_once(move || {
                                if let Some(ui) = weak.upgrade() {
                                    ui.selection_finished();
                                }
                            });
                        }
                        _ => {}
                    }
                }
                glib::Propagation::Proceed
            });
            reader.add_controller(events);
        }
        let w = self.weak();
        self.reader.buffer().connect_mark_set(move |_, _, mark| {
            if let Some(ui) = w.upgrade()
                && ui.ready.get()
                && !ui.changing.get()
                && !ui.closed.get()
                && !ui.mouse_down.get()
                && !ui.context_click.get()
                && mark.name().as_deref() == Some("selection_bound")
            {
                ui.cancel_selection();
                let w = ui.weak();
                let source = glib::timeout_add_local_once(Duration::from_millis(220), move || {
                    if let Some(ui) = w.upgrade() {
                        ui.selection_source.borrow_mut().take();
                        ui.selection_finished();
                    }
                });
                *ui.selection_source.borrow_mut() = Some(source);
            }
        });
        let w = self.weak();
        self.note_save.connect_clicked(move |_| {
            if let Some(ui) = w.upgrade() {
                let u = ui.clone();
                ui.launch(async move { u.save_note().await });
            }
        });
    }
    pub(super) fn relabel_annotations(&self) {
        let menu = gio::Menu::new();
        for (name, it, en) in [
            ("underline", "Sottolinea", "Underline"),
            ("bookmark", "Segnalibro", "Bookmark"),
            ("note", "Nota", "Note"),
            ("clear", "Rimuovi formattazione", "Clear formatting"),
        ] {
            menu.append(Some(&self.t(it, en)), Some(&format!("win.{name}")));
        }
        self.reader.set_extra_menu(Some(&menu));
        self.reader_right.set_extra_menu(Some(&menu));
        self.verse_menu.set_menu_model(Some(&menu));
        if let Some(bar) = self.popup.child() {
            for (index, it, en) in [
                (1, "Evidenzia", "Highlight"),
                (2, "Segnalibro", "Bookmark"),
                (3, "Nota", "Note"),
                (4, "Copia", "Copy"),
                (5, "Rimuovi formattazione", "Clear formatting"),
            ] {
                if let Some(widget) = bar
                    .observe_children()
                    .item(index)
                    .and_downcast::<gtk::Widget>()
                {
                    widget.set_tooltip_text(Some(&self.t(it, en)));
                }
            }
            if let Some(palette) = bar
                .observe_children()
                .item(1)
                .and_downcast::<gtk::MenuButton>()
                && let Some(colors) = palette.popover().and_then(|p| p.child())
            {
                for (index, it, en) in [
                    (0, "Giallo", "Yellow"),
                    (1, "Verde", "Green"),
                    (2, "Blu", "Blue"),
                    (3, "Rosa", "Pink"),
                ] {
                    if let Some(button) = colors
                        .observe_children()
                        .item(index)
                        .and_downcast::<gtk::Widget>()
                    {
                        button.set_tooltip_text(Some(&self.t(it, en)));
                    }
                }
            }
        }
    }
    pub(super) fn cancel_selection(&self) {
        if let Some(source) = self.selection_source.borrow_mut().take() {
            source.remove();
        }
    }
    fn capture_selection(&self) {
        let buffer = self.reader.buffer();
        let mut s = self.state.borrow_mut();
        s.selection = if let Some((a, b)) = buffer.selection_bounds() {
            model::selections(&s.verses, &s.spans, a.offset(), b.offset())
        } else {
            vec![]
        };
        let cursor = buffer.cursor_position();
        if let Some(index) = s
            .spans
            .iter()
            .position(|p| cursor >= p.prefix && cursor <= p.end)
        {
            s.position.verse = index;
        }
    }
    fn selection_finished(&self) {
        self.cancel_selection();
        if self.closed.get() || !self.ready.get() || self.changing.get() || self.context_click.get()
        {
            return;
        }
        self.capture_selection();
        self.save_position();
        if self.state.borrow().selection.is_empty() {
            self.popup.popdown();
            return;
        }
        self.point_popup(&self.popup);
        self.popup.popup();
    }
    pub(super) fn point_popup(&self, popup: &gtk::Popover) {
        let reader = if self.active_right.get() {
            &self.reader_right
        } else {
            &self.reader
        };
        if popup.parent().as_ref() != Some(reader.upcast_ref()) {
            popup.unparent();
            popup.set_parent(reader);
        }
        let s = self.state.borrow();
        let index = s
            .selection
            .first()
            .map(|p| p.verse)
            .unwrap_or(s.position.verse);
        let span = &s.spans[index];
        let iter = self.reader.buffer().iter_at_offset(span.start);
        let rect = reader.iter_location(&iter);
        let (x, y) =
            reader.buffer_to_window_coords(gtk::TextWindowType::Widget, rect.x(), rect.y());
        popup.set_pointing_to(Some(&gdk::Rectangle::new(
            x.clamp(0, reader.width().max(1)),
            y.clamp(0, reader.height().max(1)),
            1,
            rect.height().max(1),
        )));
    }
    fn annotation_action(self: &Rc<Self>, name: &str) {
        if !self.ready.get() || self.state.borrow().verses.is_empty() {
            return;
        }
        match name {
            "copy" => {
                let text = self
                    .state
                    .borrow()
                    .selection
                    .iter()
                    .map(|s| s.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                self.reader.clipboard().set_text(&text);
                self.popup.popdown();
            }
            "note" => self.show_note(),
            "bookmark" => {
                let u = self.clone();
                self.launch(async move { u.bookmark().await });
            }
            _ => {
                let u = self.clone();
                let name = name.to_owned();
                self.launch(async move { u.mark(&name).await });
            }
        }
    }
    pub(super) async fn mark(self: &Rc<Self>, action: &str) -> Result<(), String> {
        let (position, parts) = {
            let s = self.state.borrow();
            (s.position.clone(), s.selection.clone())
        };
        if parts.is_empty() {
            return Err(self.t(
                "Seleziona una parola o una frase",
                "Select a word or phrase",
            ));
        }
        self.popup.popdown();
        for part in parts {
            let value=self.core.call(json!({"op":"mark","edition":position.edition,"book":position.book,"chapter":position.chapter,"verse":part.verse,"start":part.start,"length":part.length,"action":action})).await?;
            self.update_annotation(&position, part.verse, value)?;
        }
        Ok(())
    }
    pub(super) fn update_annotation(
        &self,
        position: &Position,
        index: usize,
        value: Value,
    ) -> Result<(), String> {
        if self.closed.get() || !self.position().same_chapter(position) {
            return Ok(());
        }
        let annotation: Annotation = serde_json::from_value(value).map_err(|e| e.to_string())?;
        self.changing.set(true);
        let (prefix, bookmark, has_note) = {
            let mut s = self.state.borrow_mut();
            s.verses[index].annotation = annotation;
            (
                s.spans.get(index).map(|s| s.prefix),
                s.verses[index].annotation.bookmark,
                !s.verses[index].annotation.note.is_empty(),
            )
        };
        self.update_annotation_summary();
        let Some(prefix) = prefix else {
            self.changing.set(false);
            return Ok(());
        };
        let buffer = self.reader.buffer();
        let mut a = buffer.iter_at_offset(prefix);
        let mut b = buffer.iter_at_offset(prefix + 2);
        buffer.delete(&mut a, &mut b);
        buffer.insert(
            &mut a,
            &format!(
                "{}{}",
                if bookmark { '★' } else { ' ' },
                if has_note { '✎' } else { ' ' }
            ),
        );
        self.style_verse(index);
        self.changing.set(false);
        Ok(())
    }
    async fn bookmark(self: &Rc<Self>) -> Result<(), String> {
        let (position, index) = {
            let s = self.state.borrow();
            let index = s
                .selection
                .first()
                .map(|p| p.verse)
                .unwrap_or(s.position.verse);
            (s.position.clone(), index)
        };
        let value=self.core.call(json!({"op":"bookmark","edition":position.edition,"book":position.book,"chapter":position.chapter,"verse":index})).await?;
        self.toast
            .add_toast(adw::Toast::new(&if value["bookmark"] == true {
                self.t(
                    "Segnalibro aggiunto · Apri Segnalibri per ritrovarlo.",
                    "Bookmark added · Open Bookmarks to find it.",
                )
            } else {
                self.t("Segnalibro rimosso.", "Bookmark removed.")
            }));
        self.popup.popdown();
        self.update_annotation(&position, index, value)
    }
    pub(super) fn show_note(&self) {
        let s = self.state.borrow();
        let index = s
            .selection
            .first()
            .map(|p| p.verse)
            .unwrap_or(s.position.verse);
        let verse = &s.verses[index];
        self.note_title.set_text(&format!(
            "{} {}:{}",
            s.books[s.position.book].name,
            s.position.chapter + 1,
            verse.label
        ));
        self.note_input.buffer().set_text(&verse.annotation.note);
        self.note_color.set_selected(
            ["yellow", "green", "blue", "pink"]
                .iter()
                .position(|c| Some(*c) == verse.annotation.note_color.as_deref())
                .unwrap_or(0) as u32,
        );
        self.note_style.set_selected(u32::from(
            verse.annotation.note_style.as_deref() == Some("sticky"),
        ));
        *self.note_target.borrow_mut() = Some((s.position.clone(), index));
        drop(s);
        self.point_popup(&self.note_popup);
        self.popup.popdown();
        self.note_popup.popup();
        self.note_input.grab_focus();
    }
    async fn save_note(self: &Rc<Self>) -> Result<(), String> {
        let Some((position, index)) = self.note_target.borrow().clone() else {
            return Ok(());
        };
        let b = self.note_input.buffer();
        let text = b.text(&b.start_iter(), &b.end_iter(), false).to_string();
        let color = ["yellow", "green", "blue", "pink"][self.note_color.selected().min(3) as usize];
        let style = if self.note_style.selected() == 1 {
            "sticky"
        } else {
            "plain"
        };
        let value=self.core.call(json!({"op":"note","edition":position.edition,"book":position.book,"chapter":position.chapter,"verse":index,"text":text,"color":color,"style":style})).await?;
        self.note_popup.popdown();
        self.toast.add_toast(adw::Toast::new(&if text.is_empty() {
            self.t("Nota rimossa.", "Note removed.")
        } else {
            self.t(
                "Nota salvata · Premi la matita accanto al versetto per riaprirla.",
                "Note saved · Click the pencil beside the verse to reopen it.",
            )
        }));
        self.update_annotation(&position, index, value)
    }
}
