use super::*;

impl Ui {
    pub(super) fn comparison_changed(self: &Rc<Self>) {
        self.comparison_generation
            .set(self.comparison_generation.get() + 1);
        self.book_surface.cancel();
        self.popup.popdown();
        self.note_popup.popdown();
        self.cancel_selection();
        let active = self.comparison.is_active();
        self.book_surface.set_visible(!active);
        self.comparison_panel.set_visible(active);
        self.book_mode.set_sensitive(!active);
        self.relabel_book();
        if active {
            self.page_counter.set_text("");
            self.refresh_comparison();
        } else {
            self.comparison_data.borrow_mut().take();
            while let Some(child) = self.comparison_rows.first_child() {
                self.comparison_rows.remove(&child);
            }
            self.schedule_book();
            let cei = self.position().edition == "CEI 2008";
            self.attribution.set_visible(cei);
            self.attribution.set_text(if cei { CEI } else { "" });
        }
    }
    pub(super) fn refresh_comparison(self: &Rc<Self>) {
        if !self.comparison.is_active() || !self.ready.get() {
            return;
        }
        let p = self.position();
        let options: Vec<_> = self
            .state
            .borrow()
            .editions
            .iter()
            .filter(|e| e.id != p.edition)
            .map(|e| e.id.clone())
            .collect();
        let mut choice = self.comparison_choice.borrow().clone();
        if !options.contains(&choice) {
            choice = if options.iter().any(|x| x == "CEI 2008") {
                "CEI 2008".into()
            } else {
                options[0].clone()
            };
            self.comparison_choice.replace(choice.clone());
        }
        // Clear the callback's options while replacing the dropdown model.
        self.comparison_options.borrow_mut().clear();
        self.comparison_edition
            .set_model(Some(&gtk::StringList::new(
                &options.iter().map(String::as_str).collect::<Vec<_>>(),
            )));
        self.comparison_edition
            .set_selected(options.iter().position(|x| *x == choice).unwrap() as u32);
        self.comparison_options.replace(options);
        self.comparison_primary.set_text(&p.edition);
        self.previous.set_sensitive(p.book > 0 || p.chapter > 0);
        let s = self.state.borrow();
        self.next
            .set_sensitive(p.book + 1 < s.books.len() || p.chapter + 1 < s.books[p.book].chapters);
        drop(s);
        let key = format!("{}/{}/{}/{}", p.edition, p.book, p.chapter, choice);
        let cached = self
            .comparison_data
            .borrow()
            .as_ref()
            .filter(|(k, _)| *k == key)
            .map(|(_, data)| data.clone());
        if let Some(data) = cached {
            self.render_comparison(&data);
            return;
        }
        self.comparison_generation
            .set(self.comparison_generation.get() + 1);
        let generation = self.comparison_generation.get();
        self.comparison_notice
            .set_text(&self.t("Caricamento confronto…", "Loading comparison…"));
        while let Some(child) = self.comparison_rows.first_child() {
            self.comparison_rows.remove(&child);
        }
        let ui = self.clone();
        self.launch(async move {
            let data=ui.core.call(json!({"op":"compare","edition":p.edition,"other":choice,"book":p.book,"chapter":p.chapter})).await?;
            if ui.closed.get() || !ui.comparison.is_active() || generation!=ui.comparison_generation.get(){return Ok(());}
            ui.render_comparison(&data);
            ui.comparison_data.replace(Some((key,data)));
            Ok(())
        });
    }
    fn render_comparison(self: &Rc<Self>, data: &Value) {
        while let Some(child) = self.comparison_rows.first_child() {
            self.comparison_rows.remove(&child);
        }
        let status=match data["status"].as_str().unwrap_or("") {
            "missingBook"=>self.t("Questo libro non è presente nella seconda traduzione.","This book is absent from the second translation."),
            "differentNumbering"=>self.t("Numerazione diversa: corrispondenza non verificata per questo capitolo.","Different numbering: no verified alignment for this chapter."),
            "partial"=>self.t("Alcuni riferimenti non hanno una corrispondenza verificata. I riferimenti originali sono conservati.","Some references have no verified counterpart. Original references are preserved."),
            _=>self.t("Versetti allineati · scorrimento sincronizzato · ↗ per leggere e annotare.","Aligned verses · synchronized scrolling · ↗ to read and annotate."),
        };
        self.comparison_notice.set_text(&status);
        let cei = data["edition"] == "CEI 2008" || data["other"] == "CEI 2008";
        self.attribution.set_visible(cei);
        self.attribution.set_text(if cei { CEI } else { "" });
        let mut focus = None;
        let verse = self.position().verse;
        for row in data["rows"].as_array().into_iter().flatten() {
            let pair = gtk::Box::new(gtk::Orientation::Horizontal, 24);
            pair.set_homogeneous(true);
            margin(&pair, 12);
            for side in ["left", "right"] {
                let column = gtk::Box::new(gtk::Orientation::Vertical, 8);
                let cells = row[side].as_array().unwrap();
                if cells.is_empty() {
                    let label = gtk::Label::builder()
                        .label(self.t(
                            "Nessun versetto corrispondente verificato.",
                            "No verified matching verse.",
                        ))
                        .wrap(true)
                        .xalign(0.0)
                        .css_classes(["dim-label"])
                        .build();
                    column.append(&label);
                }
                for cell in cells {
                    let reference = gtk::Label::builder()
                        .label(cell["reference"].as_str().unwrap())
                        .xalign(0.0)
                        .css_classes(["caption", "dim-label"])
                        .build();
                    column.append(&reference);
                    let label = gtk::Label::builder()
                        .label(cell["text"].as_str().unwrap())
                        .wrap(true)
                        .wrap_mode(pango::WrapMode::WordChar)
                        .selectable(true)
                        .xalign(0.0)
                        .hexpand(true)
                        .build();
                    label.set_natural_wrap_mode(gtk::NaturalWrapMode::None);
                    let attrs = pango::AttrList::new();
                    let mut font = pango::FontDescription::from_string("Source Serif 4");
                    font.set_size(
                        (self.state.borrow().font_size * 0.75 * f64::from(pango::SCALE)) as i32,
                    );
                    attrs.insert(pango::AttrFontDesc::new(&font));
                    label.set_attributes(Some(&attrs));
                    column.append(&label);
                }
                if let Some(cell) = cells.first() {
                    let button = gtk::Button::with_label("↗");
                    button.set_halign(gtk::Align::Start);
                    button.set_tooltip_text(Some(&self.t("Apri e annota", "Open and annotate")));
                    let position = Position {
                        edition: cell["edition"].as_str().unwrap().into(),
                        book: cell["book"].as_u64().unwrap() as usize,
                        chapter: cell["chapter"].as_u64().unwrap() as usize,
                        verse: cell["verse"].as_u64().unwrap() as usize,
                    };
                    let weak = self.weak();
                    button.connect_clicked(move |_| {
                        if let Some(ui) = weak.upgrade() {
                            ui.comparison.set_active(false);
                            let u = ui.clone();
                            let p = position.clone();
                            ui.launch(async move { u.navigate(p).await });
                        }
                    });
                    column.append(&button);
                }
                pair.append(&column);
            }
            if row["left"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["verse"].as_u64() == Some(verse as u64))
            {
                focus = Some(pair.clone());
            }
            self.comparison_rows.append(&pair);
            self.comparison_rows
                .append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        }
        if let Some(row) = focus {
            let weak = self.weak();
            glib::timeout_add_local_once(Duration::from_millis(60), move || {
                if let Some(ui) = weak.upgrade()
                    && ui.comparison.is_active()
                    && let Some(rect) = row.compute_bounds(&ui.comparison_rows)
                {
                    ui.comparison_scroll
                        .vadjustment()
                        .set_value(f64::from(rect.y()));
                }
            });
        }
    }
}
