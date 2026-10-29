use super::*;

impl Ui {
    pub(super) fn preferences(self: &Rc<Self>) {
        if let Some(window) = self.prefs.borrow().upgrade() {
            window.present();
            return;
        }
        let window = adw::PreferencesWindow::builder()
            .transient_for(&self.window)
            .modal(true)
            .title(self.t("Preferenze", "Preferences"))
            .default_width(560)
            .default_height(680)
            .build();
        let page = adw::PreferencesPage::new();
        let appearance = adw::PreferencesGroup::builder()
            .title(self.t("Il tuo modo di leggere", "Your reading experience"))
            .build();
        page.add(&appearance);
        let language = adw::ComboRow::builder()
            .title(self.t("Lingua dell’app", "App language"))
            .model(&gtk::StringList::new(&["Italiano", "English"]))
            .selected(if self.state.borrow().language == "en" {
                1
            } else {
                0
            })
            .build();
        appearance.add(&language);
        let theme_value = match adw::StyleManager::default().color_scheme() {
            adw::ColorScheme::ForceLight => 1,
            adw::ColorScheme::ForceDark => 2,
            _ => 0,
        };
        let theme = adw::ComboRow::builder()
            .title(self.t("Tema", "Theme"))
            .model(&gtk::StringList::new(&[
                &self.t("Come GNOME", "Follow GNOME"),
                &self.t("Chiaro", "Light"),
                &self.t("Scuro", "Dark"),
            ]))
            .selected(theme_value)
            .build();
        appearance.add(&theme);
        let size = adw::ActionRow::builder()
            .title(self.t("Dimensione del testo", "Text size"))
            .subtitle("Source Serif 4")
            .build();
        let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 14.0, 32.0, 1.0);
        scale.set_value(self.state.borrow().font_size);
        scale.set_draw_value(true);
        scale.set_size_request(180, -1);
        size.add_suffix(&scale);
        appearance.add(&size);
        let translations=adw::PreferencesGroup::builder().title(self.t("Traduzioni","Translations")).description(self.t("La lingua dell’app è indipendente dal testo. La raccolta estesa contiene 83 libri/sezioni in inglese, inclusi 1 Enoc e Giubilei. Non è un canone universale.","App language is independent of the text. The extended collection has 83 English books/sections, including 1 Enoch and Jubilees. It is not a universal canon.")).build();
        page.add(&translations);
        let filter_value = match self.state.borrow().filter.as_str() {
            "it" => 0,
            "en" => 1,
            _ => 2,
        };
        let filter = adw::ComboRow::builder()
            .title(self.t("Lingua della Bibbia", "Bible language"))
            .model(&gtk::StringList::new(&[
                "Italiano",
                "English",
                &self.t("Tutte", "All"),
            ]))
            .selected(filter_value)
            .build();
        translations.add(&filter);
        let edition = adw::ComboRow::builder()
            .title(self.t("Traduzione", "Translation"))
            .build();
        translations.add(&edition);
        let ids = Rc::new(RefCell::new(Vec::<String>::new()));
        let changing = Rc::new(Cell::new(false));
        let refresh = {
            let weak = self.weak();
            let row = edition.clone();
            let ids = ids.clone();
            let changing = changing.clone();
            move || {
                if let Some(ui) = weak.upgrade() {
                    changing.set(true);
                    let s = ui.state.borrow();
                    let options: Vec<_> = s
                        .editions
                        .iter()
                        .filter(|e| s.filter == "all" || e.language == s.filter)
                        .collect();
                    *ids.borrow_mut() = options.iter().map(|e| e.id.clone()).collect();
                    let names: Vec<_> = options.iter().map(|e| e.id.as_str()).collect();
                    row.set_model(Some(&gtk::StringList::new(&names)));
                    row.set_selected(
                        options
                            .iter()
                            .position(|e| e.id == s.position.edition)
                            .unwrap_or(0) as u32,
                    );
                    if let Some(e) = options.get(row.selected() as usize) {
                        row.set_subtitle(&format!("{} · {}", e.books, e.language));
                    }
                    changing.set(false);
                }
            }
        };
        refresh();
        let weak = self.weak();
        let edition_ids = ids.clone();
        let edition_changing = changing.clone();
        edition.connect_selected_notify(move |row| {
            if !edition_changing.get()
                && let Some(ui) = weak.upgrade()
                && let Some(id) = edition_ids.borrow().get(row.selected() as usize).cloned()
            {
                ui.choose_edition(id);
            }
        });
        let weak = self.weak();
        let filter_ids = ids.clone();
        let filter_changing = changing.clone();
        filter.connect_selected_notify(move |row| {
            if !filter_changing.get()
                && let Some(ui) = weak.upgrade()
            {
                let value = ["it", "en", "all"][row.selected() as usize];
                ui.state.borrow_mut().filter = value.into();
                ui.core
                    .post(json!({"op":"settings","key":"bibleLanguage","value":value}));
                refresh();
                let contains = filter_ids.borrow().contains(&ui.position().edition);
                if !contains && let Some(id) = filter_ids.borrow().first().cloned() {
                    ui.choose_edition(id);
                }
            }
        });
        let weak = self.weak();
        let theme_changing = changing.clone();
        theme.connect_selected_notify(move |row| {
            if !theme_changing.get()
                && let Some(ui) = weak.upgrade()
            {
                let value = ["Default", "Light", "Dark"][row.selected() as usize];
                set_theme(value);
                ui.core
                    .post(json!({"op":"settings","key":"theme","value":value}));
            }
        });
        let weak = self.weak();
        scale.connect_value_changed(move |scale| {
            if let Some(ui) = weak.upgrade() {
                ui.set_font(scale.value());
            }
        });
        let weak = self.weak();
        let privacy=adw::PreferencesGroup::builder().title(self.t("Sul tuo dispositivo","On your device")).description(self.t("Note, segnalibri e posizione sono salvati localmente. Nessun account o tracciamento. Il testo CEI è per uso formativo e didattico non commerciale.","Notes, bookmarks and reading position stay on your device. No account or tracking. CEI text is for educational noncommercial use.")).build();
        page.add(&privacy);
        let transfers=adw::PreferencesGroup::builder().title(self.t("Esportazione e backup","Export and backup")).description(self.t("Tutti gli appunti in PDF o Markdown. Un unico backup trasferisce dati, traduzioni importate e piani tra Windows, Linux e macOS.","Export all annotations as PDF or Markdown. One backup transfers data, imported editions and plans between Windows, Linux and macOS.")).build();
        self.transfer_controls(&transfers);
        page.add(&transfers);
        let cei = gtk::LinkButton::with_label(
            "https://www.bibbiaedu.it/site/twig/templates/v3/html/notelegali/",
            "CEI 2008 · BibbiaEDU",
        );
        privacy.add(&cei);
        let appearance_weak = appearance.downgrade();
        let translations_weak = translations.downgrade();
        let privacy_weak = privacy.downgrade();
        let prefs_window = window.downgrade();
        let theme_row = theme.clone();
        let font_row = size.clone();
        let filter_row = filter.clone();
        let edition_row = edition.clone();
        language.connect_selected_notify(move |row| {
            if let Some(ui) = weak.upgrade() {
                let value = if row.selected() == 1 { "en" } else { "it" };
                ui.state.borrow_mut().language = value.into();
                ui.core
                    .post(json!({"op":"settings","key":"language","value":value}));
                ui.relabel();
                if ui.comparison.is_active(){ui.refresh_comparison();}
                changing.set(true);
                let selected_theme=theme_row.selected();
                theme_row.set_model(Some(&gtk::StringList::new(&[&ui.t("Come GNOME","Follow GNOME"),&ui.t("Chiaro","Light"),&ui.t("Scuro","Dark")])));
                theme_row.set_selected(selected_theme);
                let selected_filter=filter_row.selected();
                filter_row.set_model(Some(&gtk::StringList::new(&["Italiano","English",&ui.t("Tutte","All")])));
                filter_row.set_selected(selected_filter);
                changing.set(false);
                if let Some(group)=appearance_weak.upgrade(){group.set_title(&ui.t("Il tuo modo di leggere","Your reading experience"));}
                if let Some(group)=translations_weak.upgrade(){group.set_title(&ui.t("Traduzioni","Translations"));group.set_description(Some(&ui.t("La lingua dell’app è indipendente dal testo. La raccolta estesa contiene 83 libri/sezioni in inglese, inclusi 1 Enoc e Giubilei. Non è un canone universale.","App language is independent of the text. The extended collection has 83 English books/sections, including 1 Enoch and Jubilees. It is not a universal canon.")));}
                if let Some(group)=privacy_weak.upgrade(){group.set_title(&ui.t("Sul tuo dispositivo","On your device"));group.set_description(Some(&ui.t("Note, segnalibri e posizione sono salvati localmente. Nessun account o tracciamento. Il testo CEI è per uso formativo e didattico non commerciale.","Notes, bookmarks and reading position stay on your device. No account or tracking. CEI text is for educational noncommercial use.")));}
                row.set_title(&ui.t("Lingua dell’app", "App language"));
                theme_row.set_title(&ui.t("Tema", "Theme"));
                font_row.set_title(&ui.t("Dimensione del testo", "Text size"));
                filter_row.set_title(&ui.t("Lingua della Bibbia", "Bible language"));
                edition_row.set_title(&ui.t("Traduzione", "Translation"));
                if let Some(window) = prefs_window.upgrade() {
                    window.set_title(Some(&ui.t("Preferenze", "Preferences")));
                }
            }
        });
        window.add(&page);
        self.prefs.borrow_mut().set(Some(&window));
        window.present();
    }
    pub(super) fn set_font(self: &Rc<Self>, value: f64) {
        self.state.borrow_mut().font_size = value.clamp(14.0, 32.0);
        self.body_tag.set_size_points(value * 0.75);
        self.update_gutter();
        if let Some(source) = self.font_source.borrow_mut().take() {
            source.remove();
        }
        let weak = self.weak();
        *self.font_source.borrow_mut() = Some(glib::timeout_add_local_once(
            Duration::from_millis(150),
            move || {
                if let Some(ui) = weak.upgrade() {
                    ui.font_source.borrow_mut().take();
                    ui.core.post(json!({"op":"settings","key":"fontSize","value":ui.state.borrow().font_size as i64}));
                }
            },
        ));
    }
    pub(super) fn about(&self) {
        let about=adw::AboutWindow::builder().transient_for(&self.window).modal(true).application_name("Bibbia").application_icon(APP_ID).version(env!("CARGO_PKG_VERSION")).developer_name("Michele Serafini").website("https://github.com/Michele-Serafini/Bibbia").issue_url("https://github.com/Michele-Serafini/Bibbia/issues").license_type(gtk::License::MitX11).comments(self.t("Uno spazio tranquillo per leggere e conservare i tuoi appunti, offline. I testi biblici e i font hanno licenze separate, incluse nell’installazione.","A quiet place to read and keep your notes, offline. Bible texts and fonts have separate licenses included with the installation.")).build();
        about.add_legal_section("CEI 2008", None, gtk::License::Custom, Some(CEI));
        about.present();
    }
    pub(super) fn open_data(&self) {
        let path = crate::data_directory();
        let uri = gio::File::for_path(path).uri();
        gio::AppInfo::launch_default_for_uri_async(
            &uri,
            None::<&gio::AppLaunchContext>,
            None::<&gio::Cancellable>,
            |result| {
                if let Err(e) = result {
                    eprintln!("Bibbia: {e}");
                }
            },
        );
    }
    pub(super) fn import(self: &Rc<Self>) {
        let chooser = gtk::FileChooserNative::new(
            Some(&self.t("Importa traduzione JSON", "Import JSON translation")),
            Some(&self.window),
            gtk::FileChooserAction::Open,
            Some(&self.t("Apri", "Open")),
            Some(&self.t("Annulla", "Cancel")),
        );
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("JSON"));
        filter.add_pattern("*.json");
        chooser.add_filter(&filter);
        let weak = self.weak();
        chooser.connect_response(move |chooser, response| {
            if response == gtk::ResponseType::Accept
                && let (Some(ui), Some(path)) =
                    (weak.upgrade(), chooser.file().and_then(|f| f.path()))
            {
                ui.confirm_import(path);
            }
            chooser.destroy();
        });
        chooser.show();
    }
    fn confirm_import(self: &Rc<Self>, path: PathBuf) {
        let dialog=adw::MessageDialog::builder().transient_for(&self.window).modal(true).heading(self.t("Importa traduzione","Import translation")).body(self.t("Elenco JSON di libri, capitoli e versetti. Massimo 32 MB. La traduzione verrà copiata nel database locale.","JSON array of books, chapters and verses. Maximum 32 MB. The text will be copied to the local database.")).build();
        let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
        let name = gtk::Entry::builder()
            .placeholder_text(self.t("Nome della traduzione", "Translation name"))
            .max_length(80)
            .text(path.file_stem().and_then(|s| s.to_str()).unwrap_or(""))
            .build();
        let rights = gtk::CheckButton::with_label(&self.t(
            "Ho il diritto di utilizzare questo testo.",
            "I have the right to use this text.",
        ));
        content.append(&name);
        content.append(&rights);
        dialog.set_extra_child(Some(&content));
        dialog.add_response("cancel", &self.t("Annulla", "Cancel"));
        dialog.add_response("import", &self.t("Importa", "Import"));
        dialog.set_response_appearance("import", adw::ResponseAppearance::Suggested);
        dialog.set_response_enabled("import", false);
        dialog.set_close_response("cancel");
        let weak_dialog = dialog.downgrade();
        rights.connect_toggled(move |check| {
            if let Some(dialog) = weak_dialog.upgrade() {
                dialog.set_response_enabled("import", check.is_active());
            }
        });
        let weak = self.weak();
        dialog.connect_response(None, move |_, response| {
            if response == "import"
                && let Some(ui) = weak.upgrade()
            {
                let u = ui.clone();
                let id = name.text().to_string();
                let path = path.clone();
                ui.launch(async move {
                    let result = u
                        .core
                        .call(json!({"op":"import","id":id,"path":path}))
                        .await?;
                    let boot = u.core.call(json!({"op":"bootstrap"})).await?;
                    u.state.borrow_mut().editions =
                        serde_json::from_value(boot["editionMetadata"].clone())
                            .map_err(|e| e.to_string())?;
                    u.refresh_editions();
                    let mut position = u.position();
                    position.edition = result["id"].as_str().unwrap().into();
                    position.book = 0;
                    position.chapter = 0;
                    position.verse = 0;
                    u.stack.set_visible_child_name("read");
                    u.navigate(position).await
                });
            }
        });
        dialog.present();
    }
}
