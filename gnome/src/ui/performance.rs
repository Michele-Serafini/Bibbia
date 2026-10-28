//! Explicit, isolated native performance fixture; no polling in ordinary launches.
use super::*;
use std::time::Instant;

fn widgets(widget: &gtk::Widget) -> usize {
    let mut count = 1;
    let mut child = widget.first_child();
    while let Some(w) = child {
        count += widgets(&w);
        child = w.next_sibling();
    }
    count
}
async fn ready(ui: &Ui) -> Result<(), String> {
    for _ in 0..400 {
        let p = ui.position();
        let complete = if ui.comparison.is_active() {
            let key = format!(
                "{}/{}/{}/{}",
                p.edition,
                p.book,
                p.chapter,
                ui.comparison_choice.borrow()
            );
            ui.comparison_data
                .borrow()
                .as_ref()
                .is_some_and(|(k, _)| *k == key)
        } else {
            let pager = ui.pager.borrow();
            !pager.pending
                && !pager.pages.is_empty()
                && pager.chapter == format!("{}/{}/{}", p.edition, p.book, p.chapter)
        };
        if complete {
            glib::timeout_future(Duration::from_millis(120)).await;
            return Ok(());
        }
        glib::timeout_future(Duration::from_millis(20)).await;
    }
    Err("Performance fixture did not render".into())
}
async fn checkpoint(
    ui: &Ui,
    path: &std::path::Path,
    name: &str,
    clock: Instant,
) -> Result<Instant, String> {
    ready(ui).await?;
    let (tx, rx) = futures_channel::oneshot::channel();
    glib::unix_signal_add_local_once(10, move || {
        let _ = tx.send(());
    });
    let data = json!({"elapsed_ms":clock.elapsed().as_secs_f64()*1000.0,"widgets":widgets(ui.window.upcast_ref())});
    std::fs::write(path.join(format!("{name}.json")), data.to_string())
        .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?;
    Ok(Instant::now())
}
pub async fn run(ui: Rc<Ui>, path: PathBuf) -> Result<(), String> {
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_enable_animations(false);
    }
    ui.window.set_default_size(1000, 720);
    let mut clock = checkpoint(&ui, &path, "initial", Instant::now()).await?;
    ui.navigate(Position {
        edition: "CEI 2008".into(),
        book: 22,
        chapter: 118,
        verse: 0,
    })
    .await?;
    clock = checkpoint(&ui, &path, "long_chapter", clock).await?;
    ui.comparison_choice.replace("KJV".into());
    ui.comparison.set_active(true);
    clock = checkpoint(&ui, &path, "comparison", clock).await?;
    ui.comparison.set_active(false);
    clock = checkpoint(&ui, &path, "returned", clock).await?;
    ui.window.set_default_size(1500, 900);
    clock = checkpoint(&ui, &path, "wide", clock).await?;
    ui.window.set_default_size(1000, 720);
    clock = checkpoint(&ui, &path, "narrow", clock).await?;
    for i in 0..8 {
        ui.navigate(Position {
            edition: "CEI 2008".into(),
            book: 22,
            chapter: 117 + i % 2,
            verse: 0,
        })
        .await?;
        ready(&ui).await?;
        ui.comparison.set_active(true);
        ready(&ui).await?;
        ui.comparison.set_active(false);
        ready(&ui).await?;
    }
    clock = checkpoint(&ui, &path, "cycled", clock).await?;
    if let Some(settings) = gtk::Settings::default() {
        settings.set_gtk_enable_animations(true);
    }
    ui.next.emit_clicked();
    for _ in 0..100 {
        if ui.book_surface.is_turning() {
            break;
        }
        glib::timeout_future(Duration::from_millis(10)).await;
    }
    clock = checkpoint(&ui, &path, "animation", clock).await?;
    glib::timeout_future(Duration::from_millis(1300)).await;
    checkpoint(&ui, &path, "settled", clock).await?;
    ui.window.close();
    Ok(())
}
