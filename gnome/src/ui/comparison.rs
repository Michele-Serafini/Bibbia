//! Two native text layouts, irrespective of the chapter's verse count.
use super::*;

struct Segment {
    start: i32,
    end: i32,
    position: Position,
}
pub(super) struct Views {
    pub readers: [gtk::TextView; 2],
    pub scrolls: [gtk::ScrolledWindow; 2],
    buttons: [gtk::Button; 2],
    bodies: [gtk::TextTag; 2],
    references: [gtk::TextTag; 2],
    segments: [Vec<Segment>; 2],
    rows: [Vec<(i32, i32)>; 2],
    gaps: [Vec<gtk::TextTag>; 2],
    content_key: String,
    stamp: Option<(i32, i32, u64)>,
    row_y: Vec<i32>,
    focus: Option<usize>,
}
impl Ui {
    fn ensure_comparison(self: &Rc<Self>) {
        if self.comparison_views.borrow().is_some() {
            return;
        }
        let readers = std::array::from_fn(|_| {
            gtk::TextView::builder()
                .editable(false)
                .cursor_visible(true)
                .wrap_mode(gtk::WrapMode::WordChar)
                .left_margin(12)
                .right_margin(12)
                .top_margin(8)
                .bottom_margin(16)
                .css_classes(["bible-text"])
                .build()
        });
        let scrolls = std::array::from_fn(|i| scroll(&readers[i]));
        scrolls[0].set_policy(gtk::PolicyType::Never, gtk::PolicyType::External);
        scrolls[1].set_vadjustment(Some(&scrolls[0].vadjustment()));
        for s in &scrolls {
            s.set_overlay_scrolling(true);
        }
        let buttons = std::array::from_fn(|_| gtk::Button::with_label("↗"));
        let bodies = std::array::from_fn(|_| {
            gtk::TextTag::builder()
                .family("Source Serif 4")
                .line_height(1.2)
                .build()
        });
        let references = std::array::from_fn(|_| {
            gtk::TextTag::builder()
                .family("Adwaita Sans")
                .scale(0.65)
                .build()
        });
        let columns = gtk::Box::new(gtk::Orientation::Horizontal, 24);
        columns.set_homogeneous(true);
        columns.set_vexpand(true);
        for i in 0..2 {
            let column = gtk::Box::new(gtk::Orientation::Vertical, 8);
            column.set_vexpand(true);
            column.append(&scrolls[i]);
            buttons[i].set_halign(gtk::Align::Start);
            column.append(&buttons[i]);
            columns.append(&column);
            let table = readers[i].buffer().tag_table();
            table.add(&bodies[i]);
            table.add(&references[i]);
            let weak = self.weak();
            buttons[i].connect_clicked(move |_| {
                if let Some(ui) = weak.upgrade() {
                    ui.open_comparison(i);
                }
            });
            let weak = self.weak();
            readers[i].buffer().connect_mark_set(move |_, _, _| {
                if let Some(ui) = weak.upgrade() {
                    ui.update_comparison_buttons();
                }
            });
            let name = if i == 0 {
                "open-comparison-left"
            } else {
                "open-comparison-right"
            };
            if self.window.lookup_action(name).is_none() {
                let action = gio::SimpleAction::new(name, None);
                let weak = self.weak();
                action.connect_activate(move |_, _| {
                    if let Some(ui) = weak.upgrade() {
                        ui.open_comparison(i);
                    }
                });
                self.window.add_action(&action);
            }
        }
        let surface = book::BookSurface::new(&columns);
        surface.set_vexpand(true);
        let weak = self.weak();
        surface.on_resize(move || {
            if let Some(ui) = weak.upgrade() {
                ui.schedule_comparison();
            }
        });
        self.comparison_rows.append(&surface);
        self.comparison_views.replace(Some(Views {
            readers,
            scrolls,
            buttons,
            bodies,
            references,
            segments: std::array::from_fn(|_| Vec::new()),
            rows: std::array::from_fn(|_| Vec::new()),
            gaps: std::array::from_fn(|_| Vec::new()),
            content_key: String::new(),
            stamp: None,
            row_y: Vec::new(),
            focus: None,
        }));
    }
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
        self.verse_menu.set_sensitive(!active);
        self.relabel_book();
        if active {
            let changing = self.changing.replace(true);
            self.reader.buffer().set_text("");
            self.state.borrow_mut().spans.clear();
            self.changing.set(changing);
            self.page_counter.set_text("");
            self.ensure_comparison();
            self.refresh_comparison();
        } else {
            self.comparison_reflowing.set(true);
            self.comparison_views.borrow_mut().take();
            while let Some(child) = self.comparison_rows.first_child() {
                self.comparison_rows.remove(&child);
            }
            self.comparison_data.borrow_mut().take();
            self.comparison_reflowing.set(false);
            self.core
                .post(json!({"op":"books","edition":self.position().edition}));
            self.render();
        }
    }
    pub(super) fn refresh_comparison(self: &Rc<Self>) {
        if !self.comparison.is_active() || !self.ready.get() {
            return;
        }
        self.ensure_comparison();
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
        if *self.comparison_options.borrow() != options {
            self.comparison_options.borrow_mut().clear();
            self.comparison_edition
                .set_model(Some(&gtk::StringList::new(
                    &options.iter().map(String::as_str).collect::<Vec<_>>(),
                )));
            self.comparison_edition
                .set_selected(options.iter().position(|x| *x == choice).unwrap() as u32);
            self.comparison_options.replace(options);
        }
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
            self.render_comparison(&key, &data);
            return;
        }
        self.comparison_generation
            .set(self.comparison_generation.get() + 1);
        let generation = self.comparison_generation.get();
        self.comparison_notice
            .set_text(&self.t("Caricamento confronto…", "Loading comparison…"));
        let ui = self.clone();
        self.launch(async move{
            let data=Rc::new(ui.core.call(json!({"op":"compare","edition":p.edition,"other":choice,"book":p.book,"chapter":p.chapter})).await?);
            if ui.closed.get() || !ui.comparison.is_active() || generation!=ui.comparison_generation.get(){return Ok(());}
            ui.comparison_data.replace(Some((key.clone(),data.clone())));ui.render_comparison(&key,&data);Ok(())
        });
    }
    fn render_comparison(self: &Rc<Self>, key: &str, data: &Value) {
        let status=match data["status"].as_str().unwrap_or(""){
            "missingBook"=>self.t("Questo libro non è presente nella seconda traduzione.","This book is absent from the second translation."),
            "differentNumbering"=>self.t("Numerazione diversa: corrispondenza non verificata per questo capitolo.","Different numbering: no verified alignment for this chapter."),
            "partial"=>self.t("Alcuni riferimenti non hanno una corrispondenza verificata. I riferimenti originali sono conservati.","Some references have no verified counterpart. Original references are preserved."),
            _=>self.t("Versetti allineati · seleziona un versetto e premi ↗ per annotarlo.","Aligned verses · select a verse and press ↗ to annotate it."),
        };
        self.comparison_notice.set_text(&status);
        let cei = data["edition"] == "CEI 2008" || data["other"] == "CEI 2008";
        self.attribution.set_visible(cei);
        self.attribution.set_text(if cei { CEI } else { "" });
        let language = self.state.borrow().language.clone();
        let content_key = format!("{key}/{language}");
        self.comparison_reflowing.set(true);
        {
            let mut all = self.comparison_views.borrow_mut();
            let views = all.as_mut().unwrap();
            for side in 0..2 {
                views.bodies[side].set_size_points(self.state.borrow().font_size * 0.75);
                views.buttons[side].set_tooltip_text(Some(&self.t(
                    "Apri e annota il versetto selezionato",
                    "Open and annotate the selected verse",
                )));
                let menu = gio::Menu::new();
                menu.append(
                    Some(&self.t("Apri e annota", "Open and annotate")),
                    Some(if side == 0 {
                        "win.open-comparison-left"
                    } else {
                        "win.open-comparison-right"
                    }),
                );
                views.readers[side].set_extra_menu(Some(&menu));
                if views.content_key == content_key {
                    continue;
                }
                let buffer = views.readers[side].buffer();
                for tag in views.gaps[side].drain(..) {
                    buffer.tag_table().remove(&tag);
                }
                views.rows[side].clear();
                views.segments[side].clear();
                let mut text = String::new();
                let mut offset = 0;
                let mut refs = Vec::new();
                for row in data["rows"].as_array().unwrap() {
                    let start = offset;
                    let cells = row[if side == 0 { "left" } else { "right" }]
                        .as_array()
                        .unwrap();
                    if cells.is_empty() {
                        let missing = self.t(
                            "Nessun versetto corrispondente verificato.",
                            "No verified matching verse.",
                        );
                        offset += missing.chars().count() as i32 + 1;
                        text.push_str(&missing);
                        text.push('\n');
                    }
                    for cell in cells {
                        let segment = offset;
                        let reference = cell["reference"].as_str().unwrap();
                        text.push_str(reference);
                        text.push('\n');
                        offset += reference.chars().count() as i32 + 1;
                        refs.push((segment, offset));
                        let body = cell["text"].as_str().unwrap();
                        text.push_str(body);
                        text.push('\n');
                        offset += body.chars().count() as i32 + 1;
                        let position = Position {
                            edition: cell["edition"].as_str().unwrap().into(),
                            book: cell["book"].as_u64().unwrap() as usize,
                            chapter: cell["chapter"].as_u64().unwrap() as usize,
                            verse: cell["verse"].as_u64().unwrap() as usize,
                        };
                        views.segments[side].push(Segment {
                            start: segment,
                            end: offset,
                            position,
                        });
                    }
                    text.push('\n');
                    offset += 1;
                    views.rows[side].push((start, offset));
                }
                buffer.set_text(&text);
                let (a, b) = buffer.bounds();
                buffer.apply_tag(&views.bodies[side], &a, &b);
                for (a, b) in refs {
                    buffer.apply_tag(
                        &views.references[side],
                        &buffer.iter_at_offset(a),
                        &buffer.iter_at_offset(b),
                    );
                }
                for &(_, end) in &views.rows[side] {
                    let tag = gtk::TextTag::builder().pixels_below_lines(16).build();
                    buffer.tag_table().add(&tag);
                    buffer.apply_tag(
                        &tag,
                        &buffer.iter_at_offset(end - 1),
                        &buffer.iter_at_offset(end),
                    );
                    views.gaps[side].push(tag);
                }
            }
            if views.content_key != content_key {
                let verse = self.position().verse;
                views.focus = data["rows"].as_array().unwrap().iter().position(|r| {
                    r["left"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|v| v["verse"].as_u64() == Some(verse as u64))
                });
                if let Some(row) = views.focus {
                    for side in 0..2 {
                        let buffer = views.readers[side].buffer();
                        buffer.place_cursor(&buffer.iter_at_offset(views.rows[side][row].0));
                    }
                }
                views.content_key = content_key;
                views.row_y.clear();
            }
            views.stamp = None;
        }
        self.comparison_reflowing.set(false);
        self.update_comparison_buttons();
        self.schedule_comparison();
    }
    pub(super) fn schedule_comparison(self: &Rc<Self>) {
        if self.comparison_pending.replace(true) {
            return;
        }
        let weak = self.weak();
        glib::idle_add_local_once(move || {
            if let Some(ui) = weak.upgrade() {
                ui.comparison_pending.set(false);
                ui.reflow_comparison();
            }
        });
    }
    fn reflow_comparison(self: &Rc<Self>) {
        if !self.comparison.is_active() || self.comparison_reflowing.replace(true) {
            return;
        }
        let mut all = self.comparison_views.borrow_mut();
        let Some(views) = all.as_mut() else {
            self.comparison_reflowing.set(false);
            return;
        };
        let stamp = (
            views.readers[0].width(),
            views.readers[1].width(),
            self.state.borrow().font_size.to_bits(),
        );
        if stamp.0 < 40 || stamp.1 < 40 || views.stamp == Some(stamp) {
            self.comparison_reflowing.set(false);
            return;
        }
        let adjustment = views.scrolls[0].vadjustment();
        let old = adjustment.value();
        let anchor = views
            .row_y
            .iter()
            .rposition(|y| f64::from(*y) <= old)
            .unwrap_or(0);
        let within = old - f64::from(*views.row_y.get(anchor).unwrap_or(&0));
        for gaps in &views.gaps {
            for tag in gaps {
                tag.set_pixels_below_lines(16);
            }
        }
        let heights: [Vec<i32>; 2] = std::array::from_fn(|side| {
            let buffer = views.readers[side].buffer();
            views.rows[side]
                .iter()
                .map(|&(start, end)| {
                    views.readers[side]
                        .iter_location(&buffer.iter_at_offset(end))
                        .y()
                        - views.readers[side]
                            .iter_location(&buffer.iter_at_offset(start))
                            .y()
                })
                .collect()
        });
        for side in 0..2 {
            for (i, tag) in views.gaps[side].iter().enumerate() {
                tag.set_pixels_below_lines(16 + (heights[1 - side][i] - heights[side][i]).max(0));
            }
        }
        let buffer = views.readers[0].buffer();
        views.row_y = views.rows[0]
            .iter()
            .map(|&(start, _)| {
                views.readers[0]
                    .iter_location(&buffer.iter_at_offset(start))
                    .y()
            })
            .collect();
        for reader in &views.readers {
            reader.iter_location(&reader.buffer().end_iter());
        }
        let focus = views.focus.take();
        let target = f64::from(*views.row_y.get(focus.unwrap_or(anchor)).unwrap_or(&0))
            + if focus.is_some() {
                0.0
            } else {
                within.max(0.0)
            };
        views.stamp = Some(stamp);
        drop(all);
        self.comparison_reflowing.set(false);
        let weak = self.weak();
        glib::timeout_add_local_once(Duration::from_millis(40), move || {
            if let Some(ui) = weak.upgrade()
                && let Some(views) = ui.comparison_views.borrow().as_ref()
            {
                views.scrolls[0].vadjustment().set_value(target);
            }
        });
    }
    fn update_comparison_buttons(&self) {
        if self.comparison_reflowing.get() {
            return;
        }
        if let Ok(all) = self.comparison_views.try_borrow()
            && let Some(views) = all.as_ref()
        {
            for side in 0..2 {
                let buffer = views.readers[side].buffer();
                let cursor = buffer
                    .selection_bounds()
                    .map_or(buffer.cursor_position(), |(a, _)| a.offset());
                views.buttons[side].set_sensitive(
                    views.segments[side]
                        .iter()
                        .any(|s| s.start <= cursor && cursor < s.end),
                );
            }
        }
    }
    pub(super) fn open_comparison(self: &Rc<Self>, side: usize) {
        let position = {
            let all = self.comparison_views.borrow();
            let Some(views) = all.as_ref() else {
                return;
            };
            let buffer = views.readers[side].buffer();
            let cursor = buffer
                .selection_bounds()
                .map_or(buffer.cursor_position(), |(a, _)| a.offset());
            views.segments[side]
                .iter()
                .find(|s| s.start <= cursor && cursor < s.end)
                .map(|s| s.position.clone())
        };
        if let Some(position) = position {
            self.comparison.set_active(false);
            let ui = self.clone();
            self.launch(async move { ui.navigate(position).await });
        }
    }
    pub(super) fn comparison_row_offsets(&self) -> Vec<(i32, i32)> {
        let all = self.comparison_views.borrow();
        let Some(views) = all.as_ref() else {
            return Vec::new();
        };
        views.rows[0]
            .iter()
            .zip(&views.rows[1])
            .map(|(&(a, _), &(b, _))| {
                (
                    views.readers[0]
                        .iter_location(&views.readers[0].buffer().iter_at_offset(a))
                        .y(),
                    views.readers[1]
                        .iter_location(&views.readers[1].buffer().iter_at_offset(b))
                        .y(),
                )
            })
            .collect()
    }
}
