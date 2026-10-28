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
    // A mode/scroll change invalidates GTK's cached scene. Loaded headless
    // runners can deliver that frame after the timeout; wait for a real scene
    // instead of treating its temporarily empty cache as an application error.
    let mut node = None;
    for _ in 0..50 {
        let snapshot = gtk::Snapshot::new();
        paintable.snapshot(
            &snapshot,
            ui.window.width() as f64,
            ui.window.height() as f64,
        );
        node = snapshot.to_node();
        if node.is_some() {
            break;
        }
        ui.window.queue_draw();
        glib::timeout_future(Duration::from_millis(30)).await;
    }
    let node = node.ok_or("No rendered GTK scene")?;
    let surface = ui.window.surface().ok_or("No GTK surface")?;
    let renderer = gtk::gsk::Renderer::for_surface(&surface).ok_or("No GTK renderer")?;
    let texture = renderer.render_texture(&node, None);
    let result = texture
        .save_to_png(directory.join(name))
        .map_err(|e| e.to_string());
    renderer.unrealize();
    if name == "gnome-page-turn.png" {
        // Cairo paints unsupported transforms hot pink. A completed tick alone
        // cannot prove that the user actually saw paper instead of that block.
        let stride = texture.width() as usize * 4;
        let mut pixels = vec![0; stride * texture.height() as usize];
        texture.download(&mut pixels, stride);
        let unsupported = pixels
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] > 140 && p[2] > 140 && (p[0].min(p[2]) as i16 - p[1] as i16) > 40)
            .count();
        check(
            unsupported < pixels.len() / 4 / 20,
            "Page curl contains an unsupported renderer transform",
        )?;
    }
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

async fn native_drag(
    reader: &gtk::TextView,
    window: &adw::ApplicationWindow,
    start: i32,
) -> Result<(), String> {
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
    let rect = reader.iter_location(&reader.buffer().iter_at_offset(start));
    let (x, y) = reader.buffer_to_window_coords(gtk::TextWindowType::Widget, rect.x(), rect.y());
    let bounds = reader
        .compute_bounds(window)
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
    Ok(())
}

pub async fn run(ui: Rc<Ui>) -> Result<(), String> {
    ui.set_book_mode(false);
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
    // Theme transitions and the initial change from pages to scrolling can
    // resize the surface. Let native allocation settle before capturing it.
    glib::timeout_future(Duration::from_millis(250)).await;
    if let Some(settings) = gtk::Settings::default() {
        let initial = ui.position();
        settings.set_gtk_enable_animations(true);
        ui.next.emit_clicked();
        until(|| ui.book_surface.is_turning(), "Page turn did not start").await?;
        screenshot(&ui, "gnome-page-turn.png").await?;
        until(
            || !ui.book_surface.is_turning(),
            "Page image or frame callback was not released",
        )
        .await?;
        ui.next.emit_clicked();
        ui.next.emit_clicked();
        until(
            || ui.position().chapter == 3,
            "Rapid turns lost the latest destination",
        )
        .await?;
        until(
            || !ui.book_surface.is_turning(),
            "Rapid turn did not settle",
        )
        .await?;
        let started = ui.book_surface.turns_started();
        settings.set_gtk_enable_animations(false);
        ui.navigate(initial).await?;
        glib::timeout_future(Duration::from_millis(60)).await;
        check(
            !ui.book_surface.is_turning() && ui.book_surface.turns_started() == started,
            "Reduced motion allocated a page animation",
        )?;
        settings.set_gtk_enable_animations(true);
        println!("Book motion: curved turn, rapid navigation, reduced motion and cleanup passed");
    }
    ui.set_book_mode(true);
    until(
        || ui.pager.borrow().pages.len() > 1,
        "Book pagination did not complete",
    )
    .await?;
    check(
        ui.pager.borrow().pages.len() > 1,
        "Native book has no chapter pages",
    )?;
    let chapter = ui.position().chapter;
    ui.next.emit_clicked();
    check(
        ui.position().chapter == chapter && ui.pager.borrow().index > 0,
        "Next page skipped the chapter",
    )?;
    let saved_page = ui.pager.borrow().index;
    ui.pager.borrow_mut().chapter.clear();
    ui.book_seek.set(true);
    ui.reflow_book();
    ui.reflow_book();
    check(
        ui.pager.borrow().index == saved_page,
        "Precise book resume lost its page after deferred layout",
    )?;
    ui.previous.emit_clicked();
    check(
        ui.pager.borrow().index == 0,
        "Previous page did not restore first spread",
    )?;
    screenshot(&ui, "gnome-book.png").await?;
    ui.window.set_default_size(1400, 900);
    until(
        || ui.pager.borrow().columns == 2,
        "Wide book did not display two pages",
    )
    .await?;
    glib::timeout_future(Duration::from_millis(120)).await;
    check(
        ui.right_scroll.is_visible() && ui.right_scroll.vadjustment().value() > 0.0,
        "Facing page did not show the next text range",
    )?;
    screenshot(&ui, "gnome-spread.png").await?;
    let right_char = ui.pager.borrow().pages[1].character;
    let right_verse = ui
        .state
        .borrow()
        .spans
        .iter()
        .position(|span| span.start >= right_char)
        .unwrap();
    let right_start = ui.state.borrow().spans[right_verse].start;
    native_drag(&ui.reader_right, &ui.window, right_start).await?;
    until(
        || ui.popup.is_visible(),
        "Right page mouse selection did not open its toolbar",
    )
    .await?;
    check(
        ui.active_right.get()
            && ui
                .state
                .borrow()
                .selection
                .first()
                .is_some_and(|s| s.verse == right_verse),
        "Right page selection lost its original verse offset",
    )?;
    ui.popup.popdown();
    let buffer = ui.reader.buffer();
    buffer.place_cursor(&buffer.start_iter());

    ui.window.set_default_size(980, 800);
    ui.set_book_mode(false);
    glib::timeout_future(Duration::from_millis(180)).await;
    screenshot(&ui, "gnome-light.png").await?;
    // Real pointer events exercise capture, GTK selection, and the automatic popup.
    let start = ui.state.borrow().spans[0].start;
    native_drag(&ui.reader, &ui.window, start).await?;
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
    ui.navigate(Position {
        edition: "CEI 2008".into(),
        book: 49,
        chapter: 2,
        verse: 15,
    })
    .await?;
    ui.comparison_choice.replace("KJV".into());
    ui.comparison.set_active(true);
    until(
        || {
            ui.comparison_data
                .borrow()
                .as_ref()
                .is_some_and(|(_, v)| v["other"] == "KJV")
        },
        "Comparison did not load",
    )
    .await?;
    {
        let data = ui.comparison_data.borrow();
        let rows = &data.as_ref().unwrap().1["rows"];
        check(
            rows[15]["right"][0]["book"] == 42
                && rows[15]["right"][0]["text"]
                    .as_str()
                    .unwrap()
                    .contains("God so loved"),
            "Comparison did not align John 3:16 across different book indices",
        )?;
    }
    check(
        !ui.book_surface.is_visible() && !ui.book_mode.is_sensitive(),
        "Book paging was not suspended for comparison",
    )?;
    screenshot(&ui, "gnome-comparison.png").await?;
    let row = ui.comparison_rows.first_child().unwrap();
    let left = row.first_child().unwrap();
    let right = left.next_sibling().unwrap();
    let a = left.compute_bounds(&ui.comparison_rows).unwrap();
    let b = right.compute_bounds(&ui.comparison_rows).unwrap();
    check(
        (a.y() - b.y()).abs() < 1.0,
        "Compared verses do not share the same row",
    )?;
    let link = right
        .last_child()
        .unwrap()
        .downcast::<gtk::Button>()
        .unwrap();
    link.emit_clicked();
    until(
        || !ui.comparison.is_active() && ui.position().edition == "KJV",
        "Compared verse did not open in the annotatable reader",
    )
    .await?;
    ui.navigate(Position {
        edition: "Martini 1781 (cattolica)".into(),
        book: 0,
        chapter: 0,
        verse: 0,
    })
    .await?;
    println!(
        "GTK comparison: canonical references, aligned rows, synchronized scroll container and open-to-annotate passed"
    );
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
