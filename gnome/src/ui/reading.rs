use super::*;
struct Plans {
    ui: Weak<Ui>,
    picker: gtk::DropDown,
    calendar: gtk::Calendar,
    summary: gtk::Label,
    progress: gtk::ProgressBar,
    rows: gtk::Box,
    ids: RefCell<Vec<u64>>,
    blocked: Cell<bool>,
    generation: Cell<u64>,
}
impl Plans {
    fn selected(&self) -> Option<u64> {
        self.ids
            .borrow()
            .get(self.picker.selected() as usize)
            .copied()
    }
    async fn reload(self: &Rc<Self>) -> Result<(), String> {
        let Some(ui) = self.ui.upgrade() else {
            return Ok(());
        };
        let old = self.selected();
        let list = ui
            .core
            .call(json!({"op":"reading","action":"list"}))
            .await?;
        let mut names = Vec::new();
        let mut ids = Vec::new();
        for p in list.as_array().ok_or("Piani non validi")? {
            ids.push(p["id"].as_u64().ok_or("Piano non valido")?);
            names.push(format!(
                "{} · {}",
                p["title"].as_str().unwrap_or(""),
                p["edition"].as_str().unwrap_or("")
            ));
        }
        self.blocked.set(true);
        self.picker.set_model(Some(&gtk::StringList::new(
            &names.iter().map(String::as_str).collect::<Vec<_>>(),
        )));
        self.picker.set_selected(
            old.and_then(|id| ids.iter().position(|i| *i == id))
                .unwrap_or(0) as u32,
        );
        *self.ids.borrow_mut() = ids;
        self.blocked.set(false);
        self.detail().await
    }
    fn request_detail(self: &Rc<Self>) {
        if self.blocked.get() {
            return;
        }
        if let Some(ui) = self.ui.upgrade() {
            let this = self.clone();
            ui.launch(async move { this.detail().await });
        }
    }
    async fn detail(self: &Rc<Self>) -> Result<(), String> {
        let Some(ui) = self.ui.upgrade() else {
            return Ok(());
        };
        let generation = self.generation.get() + 1;
        self.generation.set(generation);
        while let Some(child) = self.rows.first_child() {
            self.rows.remove(&child)
        }
        self.calendar.clear_marks();
        let Some(id) = self.selected() else {
            self.summary
                .set_text(&ui.t("Crea un piano per iniziare.", "Create a plan to start."));
            self.progress.set_fraction(0.0);
            return Ok(());
        };
        let date = self.calendar.date();
        let selected = date.format("%F").map_err(|e| e.to_string())?.to_string();
        let from = format!("{:04}-{:02}-01", date.year(), date.month());
        let detail = ui
            .core
            .call(json!({"op":"reading","action":"detail","id":id,"from":from,"selected":selected}))
            .await?;
        if self.generation.get() != generation || self.selected() != Some(id) {
            return Ok(());
        }
        let total = detail["total"].as_u64().unwrap_or(1);
        let done = detail["completed"].as_u64().unwrap_or(0);
        self.progress
            .set_fraction(done as f64 / total.max(1) as f64);
        self.summary.set_text(&format!(
            "{} · {done}/{total} ({:.0}%) · {} {}",
            detail["title"].as_str().unwrap_or(""),
            done as f64 * 100.0 / total.max(1) as f64,
            detail["duration"],
            ui.t("giorni", "days")
        ));
        let month_days = glib::DateTime::from_iso8601(&format!("{from}T12:00:00Z"), None)
            .map_err(|e| e.to_string())?
            .add_months(1)
            .map_err(|e| e.to_string())?
            .add_days(-1)
            .map_err(|e| e.to_string())?
            .day_of_month();
        for d in detail["days"].as_array().ok_or("Calendario non valido")? {
            let day = d["offset"].as_i64().unwrap_or(0) + 1;
            if day <= month_days as i64
                && d["total"].as_u64().unwrap_or(0) > 0
                && d["done"] == d["total"]
            {
                self.calendar.mark_day(day as u32)
            }
        }
        for chapter in detail["chapters"].as_array().ok_or("Capitoli non validi")? {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            let check = gtk::CheckButton::with_label(&format!(
                "{} {}",
                chapter["name"].as_str().unwrap_or(""),
                chapter["chapter"].as_u64().unwrap_or(0) + 1
            ));
            check.set_hexpand(true);
            check.set_active(chapter["complete"].as_bool().unwrap_or(false));
            let index = chapter["index"].as_u64().unwrap_or(0);
            let weak = Rc::downgrade(self);
            check.connect_toggled(move|check|{if let Some(this)=weak.upgrade() && let Some(ui)=this.ui.upgrade(){let complete=check.is_active();let u=ui.clone();ui.launch(async move{u.core.call(json!({"op":"reading","action":"complete","id":id,"index":index,"complete":complete})).await?;this.detail().await});}});
            let open = gtk::Button::with_label(&ui.t("Leggi", "Read"));
            let target = Position {
                edition: detail["edition"].as_str().unwrap_or("").into(),
                book: chapter["book"].as_u64().unwrap_or(0) as usize,
                chapter: chapter["chapter"].as_u64().unwrap_or(0) as usize,
                verse: 0,
            };
            let w = ui.weak();
            open.connect_clicked(move |_| {
                if let Some(ui) = w.upgrade() {
                    ui.stack.set_visible_child_name("read");
                    let u = ui.clone();
                    let p = target.clone();
                    ui.launch(async move {
                        u.core
                            .call(json!({"op":"reading","action":"activate","id":id}))
                            .await?;
                        u.navigate(p).await
                    });
                }
            });
            row.append(&check);
            row.append(&open);
            self.rows.append(&row);
        }
        if self.rows.first_child().is_none() {
            self.rows.append(&gtk::Label::new(Some(&ui.t(
                "Nessuna lettura prevista per questo giorno.",
                "No reading scheduled for this day.",
            ))))
        }
        Ok(())
    }
}
impl Ui {
    pub(super) fn connect_reading(self: &Rc<Self>) {
        let page = gtk::Box::new(gtk::Orientation::Vertical, 12);
        margin(&page, 20);
        self.stack.add_titled_with_icon(
            &scroll(&page),
            Some("plans"),
            &self.t("Piani di lettura", "Reading plans"),
            "x-office-calendar-symbolic",
        );
        let initialized = Cell::new(false);
        let weak = self.weak();
        self.stack.connect_visible_child_name_notify(move |stack| {
            if stack.visible_child_name().as_deref() == Some("plans")
                && !initialized.replace(true)
                && let Some(ui) = weak.upgrade()
            {
                ui.build_reading(&page);
            }
        });
    }
    fn build_reading(self: &Rc<Self>, page: &gtk::Box) {
        let picker = gtk::DropDown::from_strings(&[]);
        let calendar = gtk::Calendar::new();
        calendar.set_halign(gtk::Align::Center);
        let summary = gtk::Label::new(None);
        summary.set_wrap(true);
        let progress = gtk::ProgressBar::new();
        let rows = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let resume = gtk::Button::with_label(&self.t("Riprendi", "Resume"));
        let create = gtk::Button::with_label(&self.t("Nuovo piano…", "New plan…"));
        let delete = gtk::Button::with_label(&self.t("Elimina piano…", "Delete plan…"));
        actions.append(&resume);
        actions.append(&create);
        actions.append(&delete);
        page.append(&picker);
        page.append(&summary);
        page.append(&progress);
        page.append(&actions);
        let export_actions = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let system_calendar = gtk::Button::with_label(
            &self.t("Apri nel calendario di sistema", "Open in system calendar"),
        );
        system_calendar.set_widget_name("open-plan-calendar");
        let save_calendar =
            gtk::Button::with_label(&self.t("Salva calendario .ics…", "Save calendar .ics…"));
        save_calendar.set_widget_name("save-plan-calendar");
        export_actions.append(&system_calendar);
        export_actions.append(&save_calendar);
        page.append(&export_actions);
        let explanation=gtk::Label::new(Some(&self.t("Conferma l’importazione nella tua app calendario. È una copia del piano con i progressi attuali; puoi importarla di nuovo quando il piano cambia.","Confirm the import in your calendar app. This is a snapshot of the plan and current progress; import it again when the plan changes.")));
        explanation.set_wrap(true);
        explanation.add_css_class("dim-label");
        page.append(&explanation);
        page.append(&calendar);
        page.append(&rows);
        let help=gtk::Label::new(Some(&self.t("Il punto di lettura si salva automaticamente. Segna i capitoli letti per aggiornare i progressi. I giorni completati sono marcati nel calendario.","Your reading position is saved automatically. Mark finished chapters to update progress. Completed days are marked in the calendar.")));
        help.set_wrap(true);
        help.add_css_class("dim-label");
        page.append(&help);
        let p = Rc::new(Plans {
            ui: self.weak(),
            picker,
            calendar,
            summary,
            progress,
            rows,
            ids: RefCell::new(Vec::new()),
            blocked: Cell::new(false),
            generation: Cell::new(0),
        });
        let this = p.clone();
        system_calendar.connect_clicked(move |_| {
            if let Some(id) = this.selected()
                && let Some(ui) = this.ui.upgrade()
            {
                ui.plan_calendar(id, false);
            }
        });
        let this = p.clone();
        save_calendar.connect_clicked(move |_| {
            if let Some(id) = this.selected()
                && let Some(ui) = this.ui.upgrade()
            {
                ui.plan_calendar(id, true);
            }
        });
        let weak = Rc::downgrade(&p);
        p.picker.connect_selected_notify(move |_| {
            if let Some(p) = weak.upgrade() {
                p.request_detail()
            }
        });
        let weak = Rc::downgrade(&p);
        p.calendar.connect_day_selected(move |_| {
            if let Some(p) = weak.upgrade() {
                p.request_detail()
            }
        });
        let weak = Rc::downgrade(&p);
        p.calendar.connect_next_month(move |_| {
            if let Some(p) = weak.upgrade() {
                p.request_detail()
            }
        });
        let weak = Rc::downgrade(&p);
        p.calendar.connect_prev_month(move |_| {
            if let Some(p) = weak.upgrade() {
                p.request_detail()
            }
        });
        let this = p.clone();
        self.stack.connect_visible_child_name_notify(move |stack| {
            if stack.visible_child_name().as_deref() == Some("plans")
                && let Some(ui) = this.ui.upgrade()
            {
                let p = this.clone();
                ui.launch(async move { p.reload().await });
            }
        });
        let this = p.clone();
        resume.connect_clicked(move |_| {
            if let Some(id) = this.selected()
                && let Some(ui) = this.ui.upgrade()
            {
                let u = ui.clone();
                ui.launch(async move {
                    let value = u
                        .core
                        .call(json!({"op":"reading","action":"resume","id":id}))
                        .await?;
                    let p = serde_json::from_value(value).map_err(|e| e.to_string())?;
                    u.stack.set_visible_child_name("read");
                    u.navigate(p).await
                });
            }
        });
        let this = p.clone();
        delete.connect_clicked(move |_| {
            if let Some(id) = this.selected()
                && let Some(ui) = this.ui.upgrade()
            {
                let dialog = adw::MessageDialog::new(
                    Some(&ui.window),
                    Some(&ui.t(
                        "Eliminare il piano e i suoi progressi?",
                        "Delete this plan and its progress?",
                    )),
                    None,
                );
                dialog.add_responses(&[
                    ("cancel", &ui.t("Annulla", "Cancel")),
                    ("delete", &ui.t("Elimina", "Delete")),
                ]);
                dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
                dialog.set_close_response("cancel");
                let p = this.clone();
                dialog.connect_response(None, move |dialog, response| {
                    if response == "delete"
                        && let Some(ui) = p.ui.upgrade()
                    {
                        let p = p.clone();
                        let u = ui.clone();
                        ui.launch(async move {
                            u.core
                                .call(json!({"op":"reading","action":"delete","id":id}))
                                .await?;
                            p.reload().await
                        });
                    }
                    dialog.close();
                });
                dialog.present();
            }
        });
        let this = p.clone();
        create.connect_clicked(move|button|{if let Some(ui)=this.ui.upgrade(){
            let popup=gtk::Popover::new();popup.set_parent(button);popup.connect_closed(|p|p.unparent());let box_=gtk::Box::new(gtk::Orientation::Vertical,10);margin(&box_,16);
            let title=gtk::Entry::new();title.set_text(&ui.t("La mia lettura","My reading"));title.set_max_length(100);box_.append(&title);box_.append(&gtk::Label::new(Some(&ui.position().edition)));
            let scope=gtk::CheckButton::with_label(&ui.t("Solo il libro aperto","Current book only"));box_.append(&scope);
            box_.append(&gtk::Label::new(Some(&ui.t("Data di inizio (AAAA-MM-GG)","Start date (YYYY-MM-DD)"))));let start=gtk::Entry::new();start.set_text(&glib::DateTime::now_local().unwrap().format("%F").unwrap());box_.append(&start);
            box_.append(&gtk::Label::new(Some(&ui.t("Capitoli al giorno","Chapters per day"))));let daily=gtk::SpinButton::with_range(1.0,12.0,1.0);daily.set_value(3.0);box_.append(&daily);
            let save=gtk::Button::with_label(&ui.t("Crea piano","Create plan"));box_.append(&save);let p=this.clone();let pop=popup.downgrade();let target=ui.position();save.connect_clicked(move |_|{if let Some(ui)=p.ui.upgrade(){let request=json!({"op":"reading","action":"create","edition":target.edition,"title":title.text().as_str(),"start":start.text().as_str(),"daily":daily.value_as_int(),"book":if scope.is_active(){Some(target.book)}else{None}});let p=p.clone();let u=ui.clone();let pop=pop.clone();ui.launch(async move{u.core.call(request).await?;if let Some(pop)=pop.upgrade(){pop.popdown()}p.reload().await});}});popup.set_child(Some(&box_));popup.popup();
        }});
        self.launch(async move { p.reload().await });
    }
}
