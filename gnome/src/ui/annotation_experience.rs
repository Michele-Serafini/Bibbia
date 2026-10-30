use super::*;
impl Ui {
    pub(super) fn connect_annotation_experience(self: &Rc<Self>) {
        let weak = self.weak();
        self.bookmarks.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.show_bookmarks();
            }
        });
    }
    pub(super) fn show_bookmarks(self: &Rc<Self>) {
        self.notes_changing.set(true);
        self.saved_query.set_text("");
        self.saved_folder.set_selected(0);
        self.saved_tag.set_selected(0);
        self.saved_kind.set_selected(3);
        self.saved_offset.set(0);
        self.notes_changing.set(false);
        self.stack.set_visible_child_name("saved");
        let ui = self.clone();
        self.launch(async move { ui.load_saved().await });
    }
    pub(super) fn update_annotation_summary(&self) {
        let s = self.state.borrow();
        let notes = s
            .verses
            .iter()
            .filter(|v| !v.annotation.note.is_empty())
            .count();
        let bookmarks = s.verses.iter().filter(|v| v.annotation.bookmark).count();
        let marks = s
            .verses
            .iter()
            .filter(|v| {
                v.annotation.underline
                    || v.annotation.highlight.is_some()
                    || !v.annotation.ranges.is_empty()
            })
            .count();
        let label = if s.language == "en" {
            "Chapter"
        } else {
            "Capitolo"
        };
        self.subtitle.set_text(&if notes + bookmarks + marks == 0 {
            format!("{label} {}", s.position.chapter + 1)
        } else if s.language == "en" {
            format!(
                "{label} {} · Notes: {notes} · Bookmarks: {bookmarks} · Marks: {marks}",
                s.position.chapter + 1
            )
        } else {
            format!(
                "{label} {} · Note: {notes} · Segnalibri: {bookmarks} · Segni: {marks}",
                s.position.chapter + 1
            )
        });
    }
    pub(super) fn show_bookmark(self: &Rc<Self>, index: usize) {
        let popup = gtk::Popover::new();
        let reader = if self.active_right.get() {
            &self.reader_right
        } else {
            &self.reader
        };
        popup.set_parent(reader);
        self.point_popup(&popup);
        popup.connect_closed(|p| p.unparent());
        let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
        margin(&content, 12);
        content.append(&gtk::Label::new(Some(
            &self.t("Segnalibro salvato", "Saved bookmark"),
        )));
        let all =
            gtk::Button::with_label(&self.t("Mostra tutti i segnalibri", "Show all bookmarks"));
        let remove = gtk::Button::with_label(&self.t("Rimuovi segnalibro", "Remove bookmark"));
        let weak = self.weak();
        let pop = popup.downgrade();
        all.connect_clicked(move |_| {
            if let Some(p) = pop.upgrade() {
                p.popdown();
            }
            if let Some(ui) = weak.upgrade() {
                ui.show_bookmarks();
            }
        });
        let weak = self.weak();
        let target = self.position();
        let pop = popup.downgrade();
        remove.connect_clicked(move |_|{if let Some(p)=pop.upgrade(){p.popdown();}
if let Some(ui)=weak.upgrade(){let u=ui.clone();let p=target.clone();ui.launch(async move{let value=u.core.call(json!({"op":"delete_annotation","kind":"bookmark","edition":p.edition,"book":p.book,"chapter":p.chapter,"verse":index})).await?;u.update_annotation(&p,index,value)?;u.toast.add_toast(adw::Toast::new(&u.t("Segnalibro rimosso.","Bookmark removed.")));Ok(())});}});
        content.append(&all);
        content.append(&remove);
        popup.set_child(Some(&content));
        popup.popup();
    }
    pub(super) fn add_annotation_deletion(
        self: &Rc<Self>,
        content: &gtk::Box,
        popup: &gtk::Popover,
        hit: &Hit,
    ) {
        let mut target = self.position();
        target.book = hit.book;
        target.chapter = hit.chapter;
        target.verse = hit.verse;
        for (kind, it, en) in [
            ("note", "Elimina solo la nota…", "Delete note only…"),
            (
                "all",
                "Elimina tutti gli appunti del versetto…",
                "Delete all annotations on this verse…",
            ),
        ] {
            if kind == "note" && hit.annotation.as_ref().is_none_or(|a| a.note.is_empty()) {
                continue;
            }
            let button = gtk::Button::with_label(&self.t(it, en));
            button.add_css_class("destructive-action");
            button.set_widget_name(if kind == "note" {
                "delete-note-only"
            } else {
                "delete-annotation"
            });
            content.append(&button);
            let weak = self.weak();
            let pop = popup.downgrade();
            let target = target.clone();
            button.connect_clicked(move |_|{if let Some(pop)=pop.upgrade(){pop.popdown();}
if let Some(ui)=weak.upgrade(){
                let dialog=adw::MessageDialog::new(Some(&ui.window),Some(&if kind=="note"{ui.t("Eliminare la nota?","Delete this note?")}else{ui.t("Eliminare tutti gli appunti del versetto?","Delete every annotation on this verse?")}),Some(&if kind=="note"{ui.t("Segnalibro, evidenziazioni e sottolineature saranno conservati.","The bookmark, highlights and underlines will be kept.")}else{ui.t("Verranno rimossi nota, segnalibro, segni, cartella ed etichette associati al versetto.","This removes the note, bookmark, marks, folder and tags assigned to the verse.")}));
                dialog.add_responses(&[("cancel",&ui.t("Annulla","Cancel")),("delete",&ui.t("Elimina","Delete"))]);dialog.set_response_appearance("delete",adw::ResponseAppearance::Destructive);dialog.set_default_response(Some("cancel"));dialog.set_close_response("cancel");let weak=ui.weak();let p=target.clone();
                dialog.connect_response(None,move |dialog,response|{dialog.close();if response=="delete"&&let Some(ui)=weak.upgrade(){let p=p.clone();let u=ui.clone();ui.launch(async move{let value=u.core.call(json!({"op":"delete_annotation","kind":kind,"edition":p.edition,"book":p.book,"chapter":p.chapter,"verse":p.verse})).await?;u.update_annotation(&p,p.verse,value)?;if u.saved_store.n_items()==1&&u.saved_offset.get()>=200{u.saved_offset.set(u.saved_offset.get()-200);}u.load_saved().await?;u.toast.add_toast(adw::Toast::new(&u.t("Appunti eliminati.","Annotations deleted.")));Ok(())});}});dialog.present();
            }});
        }
    }
}
