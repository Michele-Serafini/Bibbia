//! Native GTK integration tests, only enabled by the explicit fixture CLI.
use super::*;
use std::{ffi::OsStr, time::Instant};
fn check(value: bool, message: &str) -> Result<(), String> {
    if value { Ok(()) } else { Err(message.into()) }
}
async fn until(mut condition: impl FnMut() -> bool, message: &str) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition() {
        if Instant::now() > deadline {
            return Err(message.into());
        }
        glib::timeout_future(Duration::from_millis(30)).await;
    }
    Ok(())
}
async fn command(args: &[String]) -> Result<String, String> {
    let args: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
    let child = gio::Subprocess::newv(
        &args,
        gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_PIPE,
    )
    .map_err(|e| e.to_string())?;
    let (out, err) = child
        .communicate_utf8_future(None)
        .await
        .map_err(|e| e.to_string())?;
    if !child.is_successful() {
        return Err(err
            .map(|s| s.to_string())
            .unwrap_or("Test subprocess failed".into()));
    }
    Ok(out.map(|s| s.to_string()).unwrap_or_default())
}
async fn screenshot(ui: &Ui, name: &str) -> Result<(), String> {
    let Some(directory) = std::env::var_os("BIBBIA_SMOKE_SCREENSHOTS") else {
        return Ok(());
    };
    glib::timeout_future(Duration::from_millis(180)).await;
    let directory = PathBuf::from(directory);
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let paintable = gtk::WidgetPaintable::new(Some(&ui.window));
    let snapshot = gtk::Snapshot::new();
    paintable.snapshot(
        &snapshot,
        ui.window.width() as f64,
        ui.window.height() as f64,
    );
    let node = snapshot.to_node().ok_or("No rendered GTK scene")?;
    let surface = ui.window.surface().ok_or("No GTK surface")?;
    let renderer = gtk::gsk::Renderer::for_surface(&surface).ok_or("No GTK renderer")?;
    let texture = renderer.render_texture(&node, None);
    let result = texture
        .save_to_png(directory.join(name))
        .map_err(|e| e.to_string());
    renderer.unrealize();
    result
}
fn find_scale(widget: &gtk::Widget) -> Option<gtk::Scale> {
    if let Ok(scale) = widget.clone().downcast::<gtk::Scale>() {
        return Some(scale);
    }
    let mut child = widget.first_child();
    while let Some(widget) = child {
        if let Some(scale) = find_scale(&widget) {
            return Some(scale);
        }
        child = widget.next_sibling();
    }
    None
}
fn find_language(widget: &gtk::Widget) -> Option<adw::ComboRow> {
    if let Ok(row) = widget.clone().downcast::<adw::ComboRow>()
        && row.title() == "Lingua dell’app"
    {
        return Some(row);
    }
    let mut child = widget.first_child();
    while let Some(widget) = child {
        if let Some(row) = find_language(&widget) {
            return Some(row);
        }
        child = widget.next_sibling();
    }
    None
}
fn toolbar_button(ui: &Ui, index: u32) -> gtk::Button {
    ui.popup
        .child()
        .unwrap()
        .observe_children()
        .item(index)
        .unwrap()
        .downcast()
        .unwrap()
}

pub async fn run(ui: Rc<Ui>) -> Result<(), String> {
    glib::timeout_future(Duration::from_millis(400)).await;
    check(
        ui.state.borrow().verses[0].text == "Al principio creò Dio il cielo e la terra.",
        "Italian Catholic chapter was not rendered",
    )?;
    let face = ui
        .reader
        .pango_context()
        .load_font(&pango::FontDescription::from_string("Source Serif 4 16"))
        .ok_or("Reading font missing")?;
    check(
        face.describe().family().as_deref() == Some("Source Serif 4"),
        "Private Source Serif font did not load",
    )?;
    set_theme("Light");
    screenshot(&ui, "gnome-light.png").await?;
    // Real pointer events exercise capture, GTK selection, and the automatic popup.
    let id = command(&[
        "xdotool".into(),
        "search".into(),
        "--sync".into(),
        "--onlyvisible".into(),
        "--pid".into(),
        std::process::id().to_string(),
        "--name".into(),
        "^Bibbia$".into(),
    ])
    .await?;
    let id = id
        .lines()
        .next()
        .ok_or("X11 test window missing")?
        .to_owned();
    let start = ui.state.borrow().spans[0].start;
    let rect = ui
        .reader
        .iter_location(&ui.reader.buffer().iter_at_offset(start));
    let (x, y) = ui
        .reader
        .buffer_to_window_coords(gtk::TextWindowType::Widget, rect.x(), rect.y());
    let bounds = ui
        .reader
        .compute_bounds(&ui.window)
        .ok_or("Reader has no window bounds")?;
    let x = x + bounds.x() as i32 + 3;
    let y = y + bounds.y() as i32 + rect.height() / 2;
    command(&[
        "xdotool".into(),
        "mousemove".into(),
        "--window".into(),
        id.clone(),
        x.to_string(),
        y.to_string(),
        "mousedown".into(),
        "1".into(),
        "sleep".into(),
        "0.12".into(),
        "mousemove".into(),
        "--window".into(),
        id,
        (x + 120).to_string(),
        y.to_string(),
        "sleep".into(),
        "0.12".into(),
        "mouseup".into(),
        "1".into(),
    ])
    .await?;
    until(
        || ui.popup.is_visible(),
        "Mouse selection did not show the annotation popup",
    )
    .await?;
    glib::timeout_future(Duration::from_millis(160)).await;
    check(
        ui.popup.width() < 520 && ui.popup.height() < 150,
        "Annotation menu is not compact",
    )?;
    check(
        !ui.state.borrow().selection.is_empty(),
        "Mouse selection was empty",
    )?;
    ui.underline.emit_clicked();
    until(
        || {
            ui.state.borrow().verses[0]
                .annotation
                .ranges
                .iter()
                .any(|r| r.underline)
        },
        "Underline action did not persist",
    )
    .await?;
    let annotation = ui.state.borrow().verses[0].annotation.clone();
    check(
        !annotation.underline && annotation.ranges[0].length < 38,
        "Whole verse was underlined instead of the selected phrase",
    )?;
    toolbar_button(&ui, 2).emit_clicked();
    until(
        || ui.state.borrow().verses[0].annotation.bookmark,
        "Bookmark action failed",
    )
    .await?;
    gio::prelude::ActionGroupExt::activate_action(&ui.window, "note", None);
    glib::timeout_future(Duration::from_millis(120)).await;
    check(
        ui.note_popup.is_visible() && ui.note_popup.width() < 420 && ui.note_popup.height() < 360,
        "Note is not a compact popover",
    )?;
    ui.note_input.buffer().set_text("Nota GNOME: luce 🌅");
    ui.note_save.emit_clicked();
    until(
        || ui.state.borrow().verses[0].annotation.note == "Nota GNOME: luce 🌅",
        "Note action failed",
    )
    .await?;
    let old_height = ui
        .reader
        .iter_location(&ui.reader.buffer().iter_at_offset(start))
        .height();
    ui.preferences();
    let prefs = ui
        .prefs
        .borrow()
        .upgrade()
        .ok_or("Preferences window did not open")?;
    let scale = find_scale(prefs.upcast_ref()).ok_or("Native font control missing")?;
    scale.set_value(28.0);
    glib::timeout_future(Duration::from_millis(220)).await;
    check(
        ui.reader
            .iter_location(&ui.reader.buffer().iter_at_offset(start))
            .height()
            > old_height,
        "Text did not resize in the rendered document",
    )?;
    find_language(prefs.upcast_ref())
        .ok_or("App language control missing")?
        .set_selected(1);
    check(
        ui.position().edition == "Martini 1781 (cattolica)",
        "App language changed the Bible edition",
    )?;
    prefs.close();
    drop(prefs);
    set_theme("Dark");
    ui.core
        .post(json!({"op":"settings","key":"theme","value":"Dark"}));
    check(
        adw::StyleManager::default().is_dark(),
        "GNOME dark theme failed",
    )?;
    let p = Position {
        edition: "CEI 2008".into(),
        book: 0,
        chapter: 0,
        verse: 0,
    };
    ui.navigate(p).await?;
    check(
        ui.state.borrow().books.len() == 73
            && ui.state.borrow().verses[0].text == "In principio Dio creò il cielo e la terra.",
        "CEI did not render its complete Catholic catalog",
    )?;
    check(
        ui.attribution.is_visible(),
        "CEI source attribution missing",
    )?;
    ui.stack.set_visible_child_name("search");
    ui.query.set_text("Dio infatti ha tanto amato il mondo");
    ui.search_button.emit_clicked();
    until(
        || ui.search_store.n_items() > 0,
        "CEI search did not return results",
    )
    .await?;
    let hit = ui
        .search_store
        .item(0)
        .unwrap()
        .downcast::<glib::BoxedAnyObject>()
        .unwrap();
    let target = hit.borrow::<Hit>().clone();
    check(
        target.name == "Giovanni" && target.label == "16",
        "Wrong CEI search reference",
    )?;
    ui.search_view.emit_by_name::<()>("activate", &[&0_u32]);
    until(
        || {
            ui.position().book == target.book
                && ui.position().chapter == 2
                && ui.position().verse == target.verse
        },
        "Search result did not navigate to its verse",
    )
    .await?;
    ui.navigate(Position {
        edition: "Raccolta estesa (83 sezioni)".into(),
        book: 0,
        chapter: 0,
        verse: 0,
    })
    .await?;
    check(
        ui.state.borrow().books.len() == 83,
        "Extended catalog is incomplete",
    )?;
    let enoch = ui
        .state
        .borrow()
        .books
        .iter()
        .position(|b| b.name == "1 Enoch (Charles 1917)")
        .ok_or("Enoch missing")?;
    let mut p = ui.position();
    p.book = enoch;
    ui.navigate(p).await?;
    check(
        ui.state.borrow().verses[0]
            .text
            .starts_with("The words of the blessing of Enoch"),
        "Enoch did not render",
    )?;
    let jubilees = ui
        .state
        .borrow()
        .books
        .iter()
        .position(|b| b.name == "Jubilees (Charles 1917)")
        .ok_or("Jubilees missing")?;
    let mut p = ui.position();
    p.book = jubilees;
    ui.navigate(p).await?;
    check(
        ui.state.borrow().books[jubilees].chapters == 50,
        "Jubilees chapter count is wrong",
    )?;
    ui.navigate(Position {
        edition: "Martini 1781 (cattolica)".into(),
        book: 0,
        chapter: 0,
        verse: 0,
    })
    .await?;
    ui.stack.set_visible_child_name("saved");
    until(
        || ui.saved_store.n_items() == 1,
        "Saved collection did not show the annotated verse",
    )
    .await?;
    ui.saved_view.emit_by_name::<()>("activate", &[&0_u32]);
    until(
        || ui.stack.visible_child_name().as_deref() == Some("read"),
        "Saved verse did not reopen",
    )
    .await?;
    screenshot(&ui, "gnome-dark.png").await?;
    println!(
        "GTK: real mouse selection, compact popup, phrase underline, note, bookmark, native preferences, font layout, themes, languages, CEI search, extended canons and saved navigation verified"
    );
    Ok(())
}

pub async fn reopen(ui: Rc<Ui>) -> Result<(), String> {
    glib::timeout_future(Duration::from_millis(200)).await;
    let s = ui.state.borrow();
    check(
        s.language == "en" && s.font_size == 28.0,
        "Preferences did not survive restart",
    )?;
    check(
        adw::StyleManager::default().is_dark(),
        "Theme did not survive restart",
    )?;
    check(
        s.position.edition == "Martini 1781 (cattolica)",
        "Reading position did not survive restart",
    )?;
    let a = &s.verses[0].annotation;
    check(
        a.bookmark && a.note == "Nota GNOME: luce 🌅" && a.ranges.iter().any(|r| r.underline),
        "Annotations did not survive restart",
    )?;
    println!("GTK restart: phrase annotation, note, bookmark, language, font and theme verified");
    Ok(())
}
