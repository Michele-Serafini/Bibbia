use super::*;

impl Ui {
    fn note_names(&self, key: &str) -> Vec<String> {
        self.note_groups.borrow()[key]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect()
    }
    fn note_choice(&self, folder: bool) -> Option<String> {
        let index = if folder {
            self.saved_folder.selected()
        } else {
            self.saved_tag.selected()
        } as usize;
        if folder && index == 1 {
            return Some(String::new());
        }
        let first = if folder { 2 } else { 1 };
        if index < first {
            return None;
        }
        (if folder {
            &self.saved_folder
        } else {
            &self.saved_tag
        })
        .selected_item()
        .and_downcast::<gtk::StringObject>()
        .map(|s| s.string().to_string())
    }
    pub(super) fn relabel_notes(&self) {
        let folder = self.note_choice(true);
        let tag = self.note_choice(false);
        self.notes_changing.set(true);
        let folders = std::iter::once(self.t("Tutte le cartelle", "All folders"))
            .chain(std::iter::once(self.t("Senza cartella", "Unfiled")))
            .chain(self.note_names("folders"))
            .collect::<Vec<_>>();
        let tags = std::iter::once(self.t("Tutte le etichette", "All tags"))
            .chain(self.note_names("tags"))
            .collect::<Vec<_>>();
        for (view, values) in [(&self.saved_folder, &folders), (&self.saved_tag, &tags)] {
            view.set_model(Some(&gtk::StringList::new(
                &values.iter().map(String::as_str).collect::<Vec<_>>(),
            )));
        }
        self.saved_folder.set_selected(
            folder
                .as_ref()
                .and_then(|s| {
                    if s.is_empty() {
                        Some(1)
                    } else {
                        folders.iter().skip(2).position(|v| v == s).map(|i| i + 2)
                    }
                })
                .unwrap_or(0) as u32,
        );
        self.saved_tag.set_selected(
            tag.as_ref()
                .and_then(|s| tags.iter().skip(1).position(|v| v == s).map(|i| i + 1))
                .unwrap_or(0) as u32,
        );
        let old = self.saved_kind.selected().min(3);
        self.saved_kind.set_model(Some(&gtk::StringList::new(&[
            &self.t("Tutti gli appunti", "All annotations"),
            &self.t("Note", "Notes"),
            &self.t("Evidenziazioni e segni", "Highlights and marks"),
            &self.t("Segnalibri", "Bookmarks"),
        ])));
        self.saved_kind.set_selected(old);
        self.saved_query.set_placeholder_text(Some(&self.t(
            "Cerca nel testo salvato e negli appunti…",
            "Search saved text and notes…",
        )));
        self.saved_manage.set_tooltip_text(Some(
            &self.t("Gestisci cartelle ed etichette", "Manage folders and tags"),
        ));
        self.notes_changing.set(false);
    }
    pub(super) fn note_request(&self) -> Value {
        json!({"op":"saved","edition":self.position().edition,"limit":200,"offset":self.saved_offset.get(),"folder":self.note_choice(true),"tag":self.note_choice(false),"query":self.saved_query.text().to_string(),"kind":(["all","note","marked","bookmark"][self.saved_kind.selected().min(3) as usize])})
    }
    fn notes_filter_changed(self: &Rc<Self>) {
        if !self.ready.get()
            || self.notes_changing.get()
            || self.stack.visible_child_name().as_deref() != Some("saved")
        {
            return;
        }
        self.saved_offset.set(0);
        let ui = self.clone();
        self.launch(async move { ui.load_saved().await });
    }
    pub(super) fn connect_notes(self: &Rc<Self>) {
        let weak = self.weak();
        self.saved_query.connect_search_changed(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.notes_filter_changed()
            }
        });
        for control in [&self.saved_folder, &self.saved_tag, &self.saved_kind] {
            let weak = self.weak();
            control.connect_selected_notify(move |_| {
                if let Some(ui) = weak.upgrade() {
                    ui.notes_filter_changed()
                }
            });
        }
        let weak = self.weak();
        self.saved_manage.connect_clicked(move |button| {
            if let Some(ui) = weak.upgrade() {
                ui.collection_manager(button)
            }
        });
        let action = gio::SimpleAction::new(
            "organize-note",
            Some(glib::VariantTy::new("(uuu)").unwrap()),
        );
        let weak = self.weak();
        action.connect_activate(move |_, parameter| {
            if let (Some(ui), Some((b, c, v))) = (
                weak.upgrade(),
                parameter.and_then(|p| p.get::<(u32, u32, u32)>()),
            ) {
                for i in 0..ui.saved_store.n_items() {
                    if let Some(object) = ui
                        .saved_store
                        .item(i)
                        .and_downcast::<glib::BoxedAnyObject>()
                    {
                        let hit = object.borrow::<Hit>();
                        if (hit.book, hit.chapter, hit.verse)
                            == (b as usize, c as usize, v as usize)
                        {
                            ui.organize_note(&hit);
                            break;
                        }
                    }
                }
            }
        });
        self.window.add_action(&action);
    }
    pub(super) fn organize_note(self: &Rc<Self>, hit: &Hit) {
        if let Some(popup) = self.notes_popup.borrow().upgrade() {
            popup.popdown()
        }
        let popup = gtk::Popover::new();
        self.notes_popup.borrow_mut().set(Some(&popup));
        popup.set_parent(&self.saved_view);
        popup.set_autohide(true);
        popup.connect_closed(|p| p.unparent());
        let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
        margin(&content, 16);
        content.set_size_request(310, -1);
        content.append(&gtk::Label::new(Some(&format!(
            "{} {}:{}",
            hit.name,
            hit.chapter + 1,
            hit.label
        ))));
        content.append(&gtk::Label::new(Some(&self.t(
            "Cartella (vuota: senza cartella)",
            "Folder (empty: unfiled)",
        ))));
        let folder = gtk::ComboBoxText::with_entry();
        for name in self.note_names("folders") {
            folder.append_text(&name)
        }
        let input = folder.child().and_downcast::<gtk::Entry>().unwrap();
        input.set_widget_name("organize-folder");
        input.set_text(hit.organization["folder"].as_str().unwrap_or(""));
        input.set_max_length(80);
        content.append(&folder);
        content.append(&gtk::Label::new(Some(
            &self.t("Etichette separate da virgole", "Tags separated by commas"),
        )));
        let tags = gtk::Entry::new();
        tags.set_widget_name("organize-tags");
        tags.set_max_length(1000);
        tags.set_text(
            &hit.organization["tags"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", "),
        );
        content.append(&tags);
        let help = gtk::Label::new(Some(&self.t(
            "Puoi scrivere nomi nuovi. Un appunto può avere più etichette.",
            "Type new names to create them. An annotation can have several tags.",
        )));
        help.set_wrap(true);
        content.append(&help);
        let note = gtk::TextView::new();
        note.set_wrap_mode(gtk::WrapMode::WordChar);
        note.buffer().set_text(
            hit.annotation
                .as_ref()
                .map(|a| a.note.as_str())
                .unwrap_or(""),
        );
        let note_scroll = scroll(&note);
        note_scroll.set_min_content_height(110);
        content.append(&note_scroll);
        let color = gtk::DropDown::from_strings(&[
            "Giallo / Yellow",
            "Verde / Green",
            "Blu / Blue",
            "Rosa / Pink",
        ]);
        color.set_selected(
            ["yellow", "green", "blue", "pink"]
                .iter()
                .position(|c| {
                    Some(*c)
                        == hit
                            .annotation
                            .as_ref()
                            .and_then(|a| a.note_color.as_deref())
                })
                .unwrap_or(0) as u32,
        );
        let style = gtk::DropDown::from_strings(&["Nota / Note", "Post-it"]);
        style.set_selected(u32::from(
            hit.annotation
                .as_ref()
                .and_then(|a| a.note_style.as_deref())
                == Some("sticky"),
        ));
        content.append(&color);
        content.append(&style);
        let save = gtk::Button::with_label(&self.t("Salva", "Save"));
        save.set_widget_name("save-organization");
        save.add_css_class("suggested-action");
        content.append(&save);
        let weak = self.weak();
        let weak_popup = popup.downgrade();
        let p = self.position();
        let (b, c, v) = (hit.book, hit.chapter, hit.verse);
        save.connect_clicked(move |_|{if let Some(ui)=weak.upgrade(){let ui2=ui.clone();let popup=weak_popup.clone();let req=json!({"op":"organize","edition":p.edition,"book":b,"chapter":c,"verse":v,"folder":input.text().to_string(),"tags":tags.text().split(',').map(str::trim).filter(|s|!s.is_empty()).collect::<Vec<_>>()});let buffer=note.buffer();let text=buffer.text(&buffer.start_iter(),&buffer.end_iter(),false).to_string();let note_req=json!({"op":"note","edition":p.edition,"book":b,"chapter":c,"verse":v,"text":text,"color":(["yellow","green","blue","pink"][color.selected().min(3) as usize]),"style":if style.selected()==1{"sticky"}else{"plain"}});ui.launch(async move{let a=ui2.core.call(note_req).await?;if !a["note"].as_str().unwrap_or("").is_empty() || a["bookmark"]==true || a["underline"]==true || a["highlight"].is_string() || a["ranges"].as_array().is_some_and(|r|!r.is_empty()){ui2.core.call(req).await?;}
if let Some(popup)=popup.upgrade(){popup.popdown()}ui2.load_saved().await})}});
        popup.set_child(Some(&content));
        popup.popup();
    }
    fn collection_manager(self: &Rc<Self>, anchor: &gtk::Button) {
        let popup = gtk::Popover::new();
        popup.set_parent(anchor);
        popup.connect_closed(|p| p.unparent());
        let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
        margin(&content, 16);
        content.set_size_request(320, -1);
        let kind = gtk::DropDown::from_strings(&[
            &self.t("Cartelle", "Folders"),
            &self.t("Etichette", "Tags"),
        ]);
        let existing = gtk::ComboBoxText::new();
        for name in self.note_names("folders") {
            existing.append_text(&name)
        }
        let name = gtk::Entry::new();
        name.set_placeholder_text(Some(&self.t("Nome nuovo", "New name")));
        name.set_max_length(80);
        let weak = self.weak();
        let names = existing.clone();
        kind.connect_selected_notify(move |kind| {
            if let Some(ui) = weak.upgrade() {
                names.remove_all();
                for name in ui.note_names(if kind.selected() == 0 {
                    "folders"
                } else {
                    "tags"
                }) {
                    names.append_text(&name)
                }
            }
        });
        content.append(&kind);
        content.append(&existing);
        content.append(&name);
        let help = gtk::Label::new(Some(&self.t(
            "Rinominare o eliminare una raccolta conserva tutti gli appunti.",
            "Renaming or deleting a collection keeps every annotation.",
        )));
        help.set_wrap(true);
        content.append(&help);
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        for (action, it, en) in [
            ("create", "Crea", "Create"),
            ("rename", "Rinomina", "Rename"),
            ("delete", "Elimina", "Delete"),
        ] {
            let button = gtk::Button::with_label(&self.t(it, en));
            buttons.append(&button);
            let weak = self.weak();
            let popup = popup.downgrade();
            let kind = kind.clone();
            let existing = existing.clone();
            let name = name.clone();
            button.connect_clicked(move |_|{if let Some(ui)=weak.upgrade(){let ui2=ui.clone();let popup=popup.clone();let req=json!({"op":"collection","kind":if kind.selected()==0{"folder"}else{"tag"},"action":action,"name":if action=="create"{name.text().to_string()}else{existing.active_text().map(|s|s.to_string()).unwrap_or_default()},"newName":name.text().to_string()});ui.launch(async move{ui2.core.call(req).await?;if let Some(popup)=popup.upgrade(){popup.popdown()}ui2.notes_changing.set(true);ui2.saved_folder.set_selected(0);ui2.saved_tag.set_selected(0);ui2.notes_changing.set(false);ui2.saved_offset.set(0);ui2.load_saved().await})}});
        }
        content.append(&buttons);
        popup.set_child(Some(&content));
        popup.popup();
    }
}
