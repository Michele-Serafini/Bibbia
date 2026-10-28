mod assets;
mod model;
mod ui;
mod worker;
use adw::prelude::*;
use gtk::glib;
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    rc::Rc,
};

fn data_directory() -> PathBuf {
    if let Some(path) = performance_profile() {
        return path.join("notes");
    }
    let mut args = std::env::args_os();
    while let Some(arg) = args.next() {
        if arg == "--data-dir"
            && let Some(path) = args.next()
        {
            return path.into();
        }
    }
    glib::user_data_dir().join("bibbia")
}
fn performance_profile() -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os("BIBBIA_PERFORMANCE_PROFILE")?);
    (path
        .file_name()?
        .to_string_lossy()
        .starts_with("bibbia-perf-")
        && path.join(".bibbia-perf-fixture").is_file())
    .then_some(path)
}
fn benchmark_ready(ui: &Rc<ui::Ui>) {
    let Some(path) = std::env::var_os("BIBBIA_BENCHMARK_READY") else {
        return;
    };
    let frames = Cell::new(0);
    ui.window.add_tick_callback(move |_, _| {
        frames.set(frames.get() + 1);
        if frames.get() < 2 {
            return glib::ControlFlow::Continue;
        }
        let _ = std::fs::write(&path, format!("{}", std::process::id()));
        glib::ControlFlow::Break
    });
}
fn main() -> glib::ExitCode {
    let mut smoke = false;
    let mut reopen = false;
    let mut has_directory = performance_profile().is_some();
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--version") => {
                println!("Bibbia {} · GTK4/libadwaita", env!("CARGO_PKG_VERSION"));
                return glib::ExitCode::SUCCESS;
            }
            Some("--help") => {
                println!(
                    "Bibbia — offline Bible reader\nUsage: bibbia [--data-dir PATH] [--version]\nNative UI tests: --smoke-test/--smoke-reopen --data-dir ISOLATED_PATH"
                );
                return glib::ExitCode::SUCCESS;
            }
            Some("--data-dir") => {
                if !args
                    .next()
                    .is_some_and(|p| !p.is_empty() && !p.to_string_lossy().starts_with("--"))
                {
                    eprintln!("--data-dir requires a path");
                    return glib::ExitCode::FAILURE;
                }
                has_directory = true;
            }
            Some("--smoke-test") => smoke = true,
            Some("--smoke-reopen") => {
                smoke = true;
                reopen = true;
            }
            _ => {
                eprintln!("Unknown argument: {}", arg.to_string_lossy());
                return glib::ExitCode::FAILURE;
            }
        }
    }
    if smoke && !has_directory {
        eprintln!("Smoke tests require an isolated --data-dir");
        return glib::ExitCode::FAILURE;
    }
    if smoke {
        let path = data_directory();
        let marker = path.join(".bibbia-smoke-fixture");
        if (!reopen && path.join("bible.sqlite3").exists()) || (reopen && !marker.exists()) {
            eprintln!("Smoke tests require an owned, isolated fixture");
            return glib::ExitCode::FAILURE;
        }
        if !reopen
            && let Err(e) = std::fs::create_dir_all(&path)
                .and_then(|_| std::fs::write(&marker, b"Bibbia UI smoke fixture"))
        {
            eprintln!("{e}");
            return glib::ExitCode::FAILURE;
        }
    }
    assets::load_fonts();
    let app = adw::Application::builder()
        .application_id(ui::APP_ID)
        .flags(if has_directory {
            gtk::gio::ApplicationFlags::NON_UNIQUE
        } else {
            gtk::gio::ApplicationFlags::FLAGS_NONE
        })
        .build();
    app.connect_startup(|_| {
        let provider = gtk::CssProvider::new();
        provider.load_from_data(include_str!("style.css"));
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    });
    let state: Rc<RefCell<Option<Rc<ui::Ui>>>> = Rc::new(RefCell::new(None));
    let active = state.clone();
    app.connect_activate(move |app| {
        if let Some(ui) = active.borrow().as_ref() {
            ui.window.present();
            return;
        }
        let ui = ui::Ui::new(app, data_directory());
        *active.borrow_mut() = Some(ui.clone());
        let app = app.clone();
        glib::MainContext::default().spawn_local(async move {
            if let Err(e) = ui.initialize().await {
                eprintln!("Bibbia: {e}");
                if smoke {
                    ui.close();
                    app.quit();
                    std::process::exit(1);
                }
                let dialog = adw::MessageDialog::builder()
                    .transient_for(&ui.window)
                    .modal(true)
                    .heading("Impossibile aprire gli appunti / Could not open your notes")
                    .body(e)
                    .build();
                dialog.add_response("close", "Chiudi / Close");
                dialog.set_close_response("close");
                dialog.connect_response(None, move |_, _| app.quit());
                dialog.present();
            } else if let Some(path) = performance_profile() {
                if let Err(e) = ui::performance::run(ui.clone(), path).await {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            } else if smoke {
                let result = if reopen {
                    ui::smoke::reopen(ui.clone()).await
                } else {
                    ui::smoke::run(ui.clone()).await
                };
                ui.close();
                app.quit();
                if let Err(e) = result {
                    eprintln!("UI SMOKE FAILED: {e}");
                    std::process::exit(1);
                }
                println!("GNOME native UI smoke: OK");
            } else {
                benchmark_ready(&ui);
            }
        });
    });
    let ending = state.clone();
    app.connect_shutdown(move |_| {
        if let Some(ui) = ending.borrow_mut().take() {
            ui.close();
        }
    });
    app.run_with_args(&["bibbia"])
}
