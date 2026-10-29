use super::*;
use std::fs;
use std::io::{BufRead, BufReader};
impl Ui {
    fn transfer_parent(&self) -> gtk::Window {
        self.prefs
            .borrow()
            .upgrade()
            .map(|w| w.upcast())
            .unwrap_or_else(|| self.window.clone().upcast())
    }

    pub(super) fn transfer_controls(self: &Rc<Self>, group: &adw::PreferencesGroup) {
        for (action, it, en) in [
            (
                "markdown",
                "Esporta appunti Markdown…",
                "Export notes as Markdown…",
            ),
            ("pdf", "Esporta appunti PDF…", "Export notes as PDF…"),
            ("backup", "Crea backup…", "Create backup…"),
            ("restore", "Ripristina backup…", "Restore backup…"),
        ] {
            let button = gtk::Button::with_label(&self.t(it, en));
            button.set_widget_name(&format!("transfer-{action}"));
            group.add(&button);
            let weak = self.weak();
            button.connect_clicked(move |_| {
                if let Some(ui) = weak.upgrade() {
                    ui.transfer_chooser(action)
                }
            });
        }
    }
    fn transfer_chooser(self: &Rc<Self>, action: &'static str) {
        let restore = action == "restore";
        let chooser = gtk::FileChooserNative::new(
            Some(&self.t("Esportazione e backup", "Export and backup")),
            Some(&self.transfer_parent()),
            if restore {
                gtk::FileChooserAction::Open
            } else {
                gtk::FileChooserAction::Save
            },
            Some(&self.t(
                if restore { "Apri" } else { "Salva" },
                if restore { "Open" } else { "Save" },
            )),
            Some(&self.t("Annulla", "Cancel")),
        );
        chooser.set_modal(true);
        let ext = match action {
            "markdown" => "md",
            "pdf" => "pdf",
            _ => "bibbia-backup",
        };
        let filter = gtk::FileFilter::new();
        filter.add_pattern(&format!("*.{ext}"));
        filter.set_name(Some(ext));
        chooser.add_filter(&filter);
        if !restore {
            chooser.set_current_name(&format!(
                "Bibbia-{}.{ext}",
                glib::DateTime::now_local().unwrap().format("%F").unwrap()
            ));
        }
        let weak = self.weak();
        chooser.connect_response(move |chooser, response| {
            if response == gtk::ResponseType::Accept
                && let Some(ui) = weak.upgrade()
                && let Some(path) = chooser.file().and_then(|f| f.path())
            {
                let u = ui.clone();
                ui.launch(async move { u.perform_transfer(action, path).await });
            }
            chooser.destroy();
        });
        chooser.show();
    }
    async fn perform_transfer(
        self: Rc<Self>,
        action: &'static str,
        path: PathBuf,
    ) -> Result<(), String> {
        if action == "restore" {
            let summary = self
                .core
                .call(json!({"op":"transfer","action":"inspect","path":path}))
                .await?;
            let body=format!("{} {} · {} {}\n\n{}",summary["annotations"],self.t("appunti","annotations"),summary["reading_plans"],self.t("piani","plans"),self.t("Sostituisce i dati al riavvio. Una copia dei dati attuali sarà conservata nella cartella dell’app. L’app si chiuderà dopo la conferma.","Replaces your data on restart. A copy of your current data will be kept in the app’s folder. The app will close after confirmation."));
            let dialog = adw::MessageDialog::new(
                Some(&self.transfer_parent()),
                Some(&self.t("Ripristinare il backup?", "Restore backup?")),
                Some(&body),
            );
            dialog.add_responses(&[
                ("cancel", &self.t("Annulla", "Cancel")),
                (
                    "restore",
                    &self.t("Ripristina e chiudi", "Restore and close"),
                ),
            ]);
            dialog.set_response_appearance("restore", adw::ResponseAppearance::Destructive);
            dialog.set_close_response("cancel");
            let weak = self.weak();
            dialog.connect_response(None, move |dialog, response| {
                if response == "restore"
                    && let Some(ui) = weak.upgrade()
                {
                    let p = path.clone();
                    let u = ui.clone();
                    ui.launch(async move {
                        u.core
                            .call(json!({"op":"transfer","action":"restore","path":p}))
                            .await?;
                        u.window.close();
                        Ok(())
                    });
                }
                dialog.close();
            });
            dialog.present();
            return Ok(());
        }
        let busy = adw::MessageDialog::new(
            Some(&self.transfer_parent()),
            Some(&self.t("Preparazione del file…", "Preparing file…")),
            Some(&self.t(
                "L’operazione lavora sui dati salvati.",
                "Working with your saved data.",
            )),
        );
        busy.set_deletable(false);
        busy.present();
        let result = if action == "pdf" {
            let text = self.pager.borrow().directory.join(format!(
                ".export-{}.txt",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            ));
            match self
                .core
                .call(json!({"op":"export","format":"text","path":text}))
                .await
            {
                Ok(_) => gio::spawn_blocking(move || {
                    let r = write_pdf(&text, &path);
                    let _ = fs::remove_file(text);
                    r
                })
                .await
                .unwrap_or_else(|_| Err("Esportazione PDF interrotta".into())),
                Err(e) => Err(e),
            }
        } else {
            self.core
                .call(if action == "backup" {
                    json!({"op":"transfer","action":"backup","path":path})
                } else {
                    json!({"op":"export","format":"markdown","path":path})
                })
                .await
                .map(|_| ())
        };
        busy.close();
        result?;
        self.toast
            .add_toast(adw::Toast::new(&self.t("File salvato.", "File saved.")));
        Ok(())
    }
}
pub(super) fn write_pdf(text: &std::path::Path, path: &std::path::Path) -> Result<(), String> {
    let temp = path.with_file_name(format!(
        ".bibbia-pdf-{}-{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let result = (|| {
        let surface =
            gtk::cairo::PdfSurface::new(595.28, 841.89, &temp).map_err(|e| e.to_string())?;
        let context = gtk::cairo::Context::new(&surface).map_err(|e| e.to_string())?;
        let map = pangocairo::FontMap::new();
        let pango = map.create_context();
        pangocairo::functions::context_set_resolution(&pango, 72.0);
        let layout = pango::Layout::new(&pango);
        layout.set_width((499.0 * pango::SCALE as f64) as i32);
        layout.set_wrap(pango::WrapMode::WordChar);
        let mut y = 48.0;
        let mut first = true;
        for line in BufReader::new(fs::File::open(text).map_err(|e| e.to_string())?).lines() {
            let line = line.map_err(|e| e.to_string())?;
            layout.set_font_description(Some(&pango::FontDescription::from_string(if first {
                "Sans Bold 18"
            } else {
                "Sans 11"
            })));
            first = false;
            if line.is_empty() {
                y += 8.0;
                continue;
            }
            layout.set_text(&line);
            for line in layout.lines_readonly() {
                let (_, logical) = line.pixel_extents();
                let height = logical.height().max(14) as f64 + 2.0;
                if y + height > 793.0 {
                    context.show_page().map_err(|e| e.to_string())?;
                    y = 48.0;
                }
                context.move_to(48.0, y - logical.y() as f64);
                pangocairo::functions::show_layout_line(&context, &line);
                y += height;
            }
        }
        context.show_page().map_err(|e| e.to_string())?;
        surface.finish();
        surface.status().map_err(|e| e.to_string())?;
        fs::rename(&temp, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
