use super::*;
impl Ui {
    pub(super) fn plan_calendar(self: &Rc<Self>, id: u64, save: bool) {
        if save {
            let chooser = gtk::FileChooserNative::new(
                Some(&self.t("Salva calendario .ics", "Save calendar .ics")),
                Some(&self.window),
                gtk::FileChooserAction::Save,
                Some(&self.t("Salva", "Save")),
                Some(&self.t("Annulla", "Cancel")),
            );
            chooser.set_current_name("Bibbia-piano-di-lettura.ics");
            let filter = gtk::FileFilter::new();
            filter.add_pattern("*.ics");
            filter.set_name(Some("iCalendar"));
            chooser.add_filter(&filter);
            let weak = self.weak();
            chooser.connect_response(move |chooser, response| {
                if response == gtk::ResponseType::Accept
                    && let Some(ui) = weak.upgrade()
                    && let Some(mut path) = chooser.file().and_then(|f| f.path())
                {
                    if path.extension().is_none() {
                        path.set_extension("ics");
                    }
                    let u = ui.clone();
                    ui.launch(async move { u.write_plan_calendar(id, path, false).await });
                }
                chooser.destroy();
            });
            chooser.show();
        } else {
            let path = crate::data_directory().join("calendar");
            if let Err(e) = std::fs::create_dir_all(&path) {
                self.error(e);
                return;
            }
            let path = path.join(format!("Piano-{id}.ics"));
            let ui = self.clone();
            self.launch(async move { ui.write_plan_calendar(id, path, true).await });
        }
    }
    async fn write_plan_calendar(
        self: &Rc<Self>,
        id: u64,
        path: PathBuf,
        open: bool,
    ) -> Result<(), String> {
        let language = self.state.borrow().language.clone();
        self.core
            .call(
                json!({"op":"reading","action":"calendar","id":id,"path":path,"language":language}),
            )
            .await?;
        if open {
            let file = gio::File::for_path(&path);
            gio::AppInfo::launch_default_for_uri_future(&file.uri(),None::<&gio::AppLaunchContext>).await.map_err(|e|format!("{} {e}",self.t("Scegli un’app per i file .ics, oppure usa Salva calendario .ics… per importarli manualmente.","Choose an app for .ics files, or use Save calendar .ics… to import manually.")))?;
        } else {
            self.toast.add_toast(adw::Toast::new(&self.t(
                "Calendario salvato · Importalo nella tua app calendario.",
                "Calendar saved · Import it into your calendar app.",
            )));
        }
        Ok(())
    }
}
