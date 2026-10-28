use crate::{
    model::{self, Book, Edition, Hit, Position, Selection, Span, Verse},
    worker::Worker,
};
use adw::prelude::*;
use gtk::{gdk, gio, glib, pango};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    path::PathBuf,
    rc::{Rc, Weak},
    time::Duration,
};

mod annotations;
mod book;
mod book_math;
mod comparison;
mod paging;
pub mod performance;
mod preferences;
pub mod smoke;

pub const APP_ID: &str = "io.github.MicheleSerafini.Bibbia";
const CEI: &str = "CEI 2008 · © 2008–2024 Fondazione di Religione Santi Francesco d’Assisi e Caterina da Siena. Uso formativo e didattico non commerciale · BibbiaEDU. Ester: G = greco, E = ebraico.";

struct State {
    position: Position,
    language: String,
    filter: String,
    font_size: f64,
    editions: Vec<Edition>,
    books: Rc<Vec<Book>>,
    catalogs: HashMap<String, Rc<Vec<Book>>>,
    verses: Vec<Verse>,
    spans: Vec<Span>,
    selection: Vec<Selection>,
}

pub struct Ui {
    pub window: adw::ApplicationWindow,
    core: Worker,
    state: RefCell<State>,
    ready: Cell<bool>,
    closed: Cell<bool>,
    changing: Cell<bool>,
    generation: Cell<u64>,
    pending_position: RefCell<Option<Position>>,
    search_generation: Cell<u64>,
    saved_offset: Cell<usize>,
    mouse_down: Cell<bool>,
    context_click: Cell<bool>,
    selection_source: RefCell<Option<glib::SourceId>>,
    font_source: RefCell<Option<glib::SourceId>>,
    toast: adw::ToastOverlay,
    stack: adw::ViewStack,
    header_title: adw::ViewSwitcherTitle,
    menu: gtk::MenuButton,
    verse_menu: gtk::MenuButton,
    edition: gtk::DropDown,
    book: gtk::DropDown,
    chapter: gtk::SpinButton,
    reference: gtk::Label,
    subtitle: gtk::Label,
    attribution: gtk::Label,
    previous: gtk::Button,
    next: gtk::Button,
    reader: gtk::TextView,
    reader_right: gtk::TextView,
    read_scroll: gtk::ScrolledWindow,
    right_scroll: gtk::ScrolledWindow,
    read_panes: gtk::Fixed,
    book_mode: gtk::ToggleButton,
    page_counter: gtk::Label,
    pager: RefCell<paging::Pager>,
    book_seek: Cell<bool>,
    active_right: Cell<bool>,
    comparison: gtk::ToggleButton,
    comparison_panel: gtk::Box,
    comparison_primary: gtk::Label,
    comparison_edition: gtk::DropDown,
    comparison_notice: gtk::Label,
    comparison_rows: gtk::Box,
    comparison_views: RefCell<Option<comparison::Views>>,
    comparison_pending: Cell<bool>,
    comparison_reflowing: Cell<bool>,
    comparison_choice: RefCell<String>,
    comparison_options: RefCell<Vec<String>>,
    comparison_generation: Cell<u64>,
    comparison_data: RefCell<Option<(String, Rc<Value>)>>,
    book_surface: book::BookSurface,
    body_tag: gtk::TextTag,
    number_tag: gtk::TextTag,
    popup: gtk::Popover,
    note_popup: gtk::Popover,
    underline: gtk::Button,
    note_input: gtk::TextView,
    note_save: gtk::Button,
    note_title: gtk::Label,
    note_target: RefCell<Option<(Position, usize)>>,
    query: gtk::SearchEntry,
    search_button: gtk::Button,
    search_status: gtk::Label,
    search_store: gio::ListStore,
    search_view: gtk::ListView,
    saved_status: gtk::Label,
    saved_store: gio::ListStore,
    saved_view: gtk::ListView,
    saved_previous: gtk::Button,
    saved_next: gtk::Button,
    prefs: RefCell<glib::WeakRef<adw::PreferencesWindow>>,
}

fn icon_button(icon: &str, label: &str) -> gtk::Button {
    gtk::Button::builder()
        .icon_name(icon)
        .tooltip_text(label)
        .build()
}
fn margin(widget: &impl IsA<gtk::Widget>, value: i32) {
    widget.set_margin_top(value);
    widget.set_margin_bottom(value);
    widget.set_margin_start(value);
    widget.set_margin_end(value);
}
fn scroll(child: &impl IsA<gtk::Widget>) -> gtk::ScrolledWindow {
    gtk::ScrolledWindow::builder()
        .child(child)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build()
}
fn results() -> (gio::ListStore, gtk::ListView) {
    let store = gio::ListStore::new::<glib::BoxedAnyObject>();
    let selection = gtk::NoSelection::new(Some(store.clone()));
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let row = gtk::Box::new(gtk::Orientation::Vertical, 6);
        margin(&row, 16);
        let title = gtk::Label::builder()
            .xalign(0.0)
            .css_classes(["heading"])
            .build();
        let text = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .wrap_mode(pango::WrapMode::WordChar)
            .lines(3)
            .ellipsize(pango::EllipsizeMode::End)
            .build();
        let note = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .lines(2)
            .ellipsize(pango::EllipsizeMode::End)
            .css_classes(["dim-label"])
            .build();
        row.append(&title);
        row.append(&text);
        row.append(&note);
        item.set_child(Some(&row));
    });
    factory.connect_bind(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().unwrap();
        let object = item
            .item()
            .unwrap()
            .downcast::<glib::BoxedAnyObject>()
            .unwrap();
        let hit = object.borrow::<Hit>();
        let row = item.child().unwrap();
        let title = row.first_child().unwrap().downcast::<gtk::Label>().unwrap();
        let text = title
            .next_sibling()
            .unwrap()
            .downcast::<gtk::Label>()
            .unwrap();
        let note = text
            .next_sibling()
            .unwrap()
            .downcast::<gtk::Label>()
            .unwrap();
        title.set_text(&format!("{} {}:{}", hit.name, hit.chapter + 1, hit.label));
        text.set_text(&hit.text);
        let message = hit
            .annotation
            .as_ref()
            .map(|a| a.note.as_str())
            .unwrap_or("");
        note.set_text(message);
        note.set_visible(!message.is_empty());
    });
    let view = gtk::ListView::new(Some(selection), Some(factory));
    view.set_single_click_activate(true);
    view.add_css_class("navigation-sidebar");
    (store, view)
}

impl Ui {
    pub fn new(app: &adw::Application, directory: PathBuf) -> Rc<Self> {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Bibbia")
            .default_width(980)
            .default_height(800)
            .build();
        window.add_css_class("bibbia");
        let style = adw::StyleManager::default();
        if style.is_dark() {
            window.add_css_class("book-dark");
        }
        let weak_window = window.downgrade();
        style.connect_dark_notify(move |style| {
            if let Some(window) = weak_window.upgrade() {
                if style.is_dark() {
                    window.add_css_class("book-dark");
                } else {
                    window.remove_css_class("book-dark");
                }
            }
        });
        window.set_icon_name(Some(APP_ID));
        let stack = adw::ViewStack::new();
        stack.set_vexpand(true);
        let header_title = adw::ViewSwitcherTitle::builder()
            .title("Bibbia")
            .stack(&stack)
            .build();
        let header = adw::HeaderBar::builder()
            .title_widget(&header_title)
            .build();
        let menu = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .tooltip_text("Menu")
            .build();
        header.pack_end(&menu);
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.append(&header);
        let toast = adw::ToastOverlay::new();
        toast.set_child(Some(&stack));
        toast.set_vexpand(true);
        root.append(&toast);
        let switcher = adw::ViewSwitcherBar::builder().stack(&stack).build();
        root.append(&switcher);
        header_title
            .bind_property("title-visible", &switcher, "reveal")
            .sync_create()
            .build();
        window.set_content(Some(&root));

        let read = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        margin(&controls, 16);
        let book = gtk::DropDown::from_strings(&[]);
        book.set_hexpand(true);
        book.set_enable_search(true);
        book.set_tooltip_text(Some("Libro"));
        let chapter = gtk::SpinButton::with_range(1.0, 150.0, 1.0);
        chapter.set_numeric(true);
        chapter.set_width_chars(3);
        chapter.set_tooltip_text(Some("Capitolo"));
        controls.append(&book);
        controls.append(&chapter);
        let top_clamp = adw::Clamp::builder()
            .maximum_size(1640)
            .child(&controls)
            .build();
        read.append(&top_clamp);
        let paper = gtk::Box::new(gtk::Orientation::Vertical, 8);
        paper.add_css_class("book-paper");
        let reference = gtk::Label::builder()
            .label("Bibbia")
            .css_classes(["title-1"])
            .margin_top(20)
            .build();
        let subtitle = gtk::Label::builder()
            .css_classes(["dim-label"])
            .margin_bottom(12)
            .build();
        paper.append(&reference);
        paper.append(&subtitle);
        let reader = gtk::TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .wrap_mode(gtk::WrapMode::WordChar)
            .left_margin(24)
            .right_margin(24)
            .top_margin(8)
            .bottom_margin(24)
            .pixels_below_lines(12)
            .build();
        reader.add_css_class("bible-text");
        let body_tag = gtk::TextTag::builder()
            .name("body")
            .family("Source Serif 4")
            .size_points(16.5)
            .line_height(1.2)
            .build();
        let number_tag = gtk::TextTag::builder()
            .name("number")
            .family("Adwaita Sans")
            .scale(0.65)
            .build();
        let table = reader.buffer().tag_table();
        table.add(&body_tag);
        table.add(&number_tag);
        let underline_tag = gtk::TextTag::builder()
            .name("underline")
            .underline(pango::Underline::Single)
            .build();
        table.add(&underline_tag);
        for (name, color) in [
            ("yellow", "#f4d35e"),
            ("green", "#a9d99b"),
            ("blue", "#99c1f1"),
            ("pink", "#f5c2e7"),
        ] {
            table.add(
                &gtk::TextTag::builder()
                    .name(name)
                    .background(color)
                    .foreground("#1b1b1b")
                    .build(),
            );
        }
        let scrolled = scroll(&reader);
        let reader_right = gtk::TextView::builder()
            .buffer(&reader.buffer())
            .editable(false)
            .cursor_visible(false)
            .wrap_mode(gtk::WrapMode::WordChar)
            .left_margin(24)
            .right_margin(24)
            .top_margin(8)
            .bottom_margin(24)
            .pixels_below_lines(12)
            .build();
        reader_right.add_css_class("bible-text");
        let right_scroll = scroll(&reader_right);
        let read_panes = gtk::Fixed::new();
        read_panes.set_hexpand(true);
        read_panes.set_vexpand(true);
        read_panes.put(&scrolled, 0.0, 0.0);
        read_panes.put(&right_scroll, 0.0, 0.0);
        let book_mode = gtk::ToggleButton::with_label("Libro");
        let page_counter = gtk::Label::new(None);
        let mut pager = paging::Pager {
            directory: directory.clone(),
            ..Default::default()
        };
        if let Ok(data) = std::fs::read(directory.join("reader.json"))
            && let Ok(place) = serde_json::from_slice(&data)
        {
            pager.place = place;
        }
        book_mode.set_active(pager.place.book_mode);
        let book_surface = book::BookSurface::new(&read_panes);
        paper.append(&book_surface);
        let comparison = gtk::ToggleButton::with_label("Confronta");
        let comparison_panel = gtk::Box::new(gtk::Orientation::Vertical, 12);
        margin(&comparison_panel, 16);
        comparison_panel.set_vexpand(true);
        comparison_panel.set_visible(false);
        let comparison_header = gtk::Box::new(gtk::Orientation::Horizontal, 24);
        comparison_header.set_homogeneous(true);
        let comparison_primary = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .css_classes(["heading"])
            .build();
        let comparison_edition = gtk::DropDown::from_strings(&[]);
        comparison_header.append(&comparison_primary);
        comparison_header.append(&comparison_edition);
        comparison_panel.append(&comparison_header);
        let comparison_notice = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .css_classes(["caption", "dim-label"])
            .build();
        comparison_panel.append(&comparison_notice);
        let comparison_rows = gtk::Box::new(gtk::Orientation::Vertical, 0);
        comparison_rows.set_vexpand(true);
        comparison_panel.append(&comparison_rows);
        paper.append(&comparison_panel);
        paper.set_vexpand(true);
        let clamp = adw::Clamp::builder()
            .maximum_size(1640)
            .tightening_threshold(600)
            .child(&paper)
            .vexpand(true)
            .build();
        read.append(&clamp);
        clamp.set_margin_start(18);
        clamp.set_margin_end(18);
        clamp.set_margin_bottom(12);
        let footer = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        margin(&footer, 12);
        let previous = icon_button("go-previous-symbolic", "Capitolo precedente");
        let next = icon_button("go-next-symbolic", "Capitolo successivo");
        previous.add_css_class("book-nav");
        next.add_css_class("book-nav");
        let edition = gtk::DropDown::from_strings(&[]);
        edition.set_hexpand(true);
        edition.set_tooltip_text(Some("Traduzione"));
        let verse_menu = gtk::MenuButton::builder()
            .icon_name("document-edit-symbolic")
            .tooltip_text("Annotazioni del versetto")
            .build();
        footer.append(&book_mode);
        footer.append(&comparison);
        footer.append(&page_counter);
        footer.append(&previous);
        footer.append(&edition);
        footer.append(&verse_menu);
        footer.append(&next);
        read.append(
            &adw::Clamp::builder()
                .maximum_size(1640)
                .child(&footer)
                .build(),
        );
        let attribution = gtk::Label::builder()
            .wrap(true)
            .css_classes(["caption", "dim-label"])
            .margin_start(20)
            .margin_end(20)
            .margin_bottom(10)
            .build();
        read.append(&attribution);
        stack.add_titled_with_icon(
            &read,
            Some("read"),
            "Leggi",
            "accessories-dictionary-symbolic",
        );

        let search = gtk::Box::new(gtk::Orientation::Vertical, 12);
        margin(&search, 16);
        let searchbar = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let query = gtk::SearchEntry::builder()
            .placeholder_text("Una parola o una frase…")
            .hexpand(true)
            .build();
        let search_button = gtk::Button::with_label("Cerca");
        search_button.add_css_class("suggested-action");
        searchbar.append(&query);
        searchbar.append(&search_button);
        search.append(&searchbar);
        let search_status = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .css_classes(["dim-label"])
            .build();
        search.append(&search_status);
        let (search_store, search_view) = results();
        search.append(&scroll(&search_view));
        stack.add_titled_with_icon(&search, Some("search"), "Cerca", "system-search-symbolic");
        let saved = gtk::Box::new(gtk::Orientation::Vertical, 12);
        margin(&saved, 16);
        let saved_status = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .css_classes(["dim-label"])
            .build();
        saved.append(&saved_status);
        let (saved_store, saved_view) = results();
        saved.append(&scroll(&saved_view));
        let pages = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        pages.set_halign(gtk::Align::Center);
        let saved_previous = icon_button("go-previous-symbolic", "Pagina precedente");
        let saved_next = icon_button("go-next-symbolic", "Pagina successiva");
        pages.append(&saved_previous);
        pages.append(&saved_next);
        saved.append(&pages);
        stack.add_titled_with_icon(
            &saved,
            Some("saved"),
            "La mia raccolta",
            "user-bookmarks-symbolic",
        );
        let popup = gtk::Popover::new();
        popup.set_parent(&reader);
        popup.set_position(gtk::PositionType::Top);
        let underline = icon_button("format-text-underline-symbolic", "Sottolinea");
        let note_popup = gtk::Popover::new();
        note_popup.set_parent(&reader);
        note_popup.set_position(gtk::PositionType::Top);
        let notes = gtk::Box::new(gtk::Orientation::Vertical, 12);
        margin(&notes, 12);
        notes.set_size_request(300, -1);
        let note_title = gtk::Label::builder()
            .xalign(0.0)
            .css_classes(["heading"])
            .build();
        notes.append(&note_title);
        let note_input = gtk::TextView::builder()
            .wrap_mode(gtk::WrapMode::WordChar)
            .accepts_tab(false)
            .build();
        let note_scroll = scroll(&note_input);
        note_scroll.set_min_content_height(120);
        note_scroll.set_max_content_height(220);
        notes.append(&note_scroll);
        let note_save = gtk::Button::with_label("Salva nota");
        note_save.add_css_class("suggested-action");
        notes.append(&note_save);
        note_popup.set_child(Some(&notes));
        let ui = Rc::new(Self {
            window,
            core: Worker::new(directory),
            state: RefCell::new(State {
                position: Position {
                    edition: "Martini 1781 (cattolica)".into(),
                    book: 0,
                    chapter: 0,
                    verse: 0,
                },
                language: "it".into(),
                filter: "it".into(),
                font_size: 22.0,
                editions: vec![],
                books: Rc::new(vec![]),
                catalogs: HashMap::new(),
                verses: vec![],
                spans: vec![],
                selection: vec![],
            }),
            ready: Cell::new(false),
            closed: Cell::new(false),
            changing: Cell::new(false),
            generation: Cell::new(0),
            pending_position: RefCell::new(None),
            search_generation: Cell::new(0),
            saved_offset: Cell::new(0),
            mouse_down: Cell::new(false),
            context_click: Cell::new(false),
            selection_source: RefCell::new(None),
            font_source: RefCell::new(None),
            toast,
            stack,
            header_title,
            menu,
            verse_menu: verse_menu.clone(),
            edition,
            book,
            chapter,
            reference,
            subtitle,
            attribution,
            previous,
            next,
            reader,
            reader_right,
            read_scroll: scrolled,
            right_scroll,
            read_panes,
            book_mode,
            page_counter,
            pager: RefCell::new(pager),
            book_seek: Cell::new(false),
            active_right: Cell::new(false),
            comparison,
            comparison_panel,
            comparison_primary,
            comparison_edition,
            comparison_notice,
            comparison_rows,
            comparison_views: RefCell::new(None),
            comparison_pending: Cell::new(false),
            comparison_reflowing: Cell::new(false),
            comparison_choice: RefCell::new("CEI 2008".into()),
            comparison_options: RefCell::new(Vec::new()),
            comparison_generation: Cell::new(0),
            comparison_data: RefCell::new(None),
            book_surface,
            body_tag,
            number_tag,
            popup,
            note_popup,
            underline,
            note_input,
            note_save,
            note_title,
            note_target: RefCell::new(None),
            query,
            search_button,
            search_status,
            search_store,
            search_view,
            saved_status,
            saved_store,
            saved_view,
            saved_previous,
            saved_next,
            prefs: RefCell::new(glib::WeakRef::new()),
        });
        let weak = ui.weak();
        ui.book_surface.on_resize(move || {
            if let Some(ui) = weak.upgrade() {
                ui.schedule_book();
            }
        });
        let weak = ui.weak();
        ui.comparison.connect_toggled(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.comparison_changed();
            }
        });
        let weak = ui.weak();
        ui.comparison_edition
            .connect_selected_notify(move |picker| {
                if let Some(ui) = weak.upgrade() {
                    let choice = ui
                        .comparison_options
                        .borrow()
                        .get(picker.selected() as usize)
                        .cloned();
                    let changed = choice
                        .as_ref()
                        .is_some_and(|choice| *ui.comparison_choice.borrow() != *choice);
                    if changed && let Some(choice) = choice {
                        ui.comparison_choice.replace(choice.clone());
                        ui.core.post(
                            json!({"op":"settings","key":"comparisonEdition","value":choice}),
                        );
                        ui.refresh_comparison();
                    }
                }
            });
        let weak = ui.weak();
        ui.book_mode.connect_toggled(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.book_surface.cancel();
                ui.schedule_book();
                ui.save_book();
                ui.relabel_book();
            }
        });
        let wheel = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        wheel.set_propagation_phase(gtk::PropagationPhase::Capture);
        let last = Rc::new(Cell::new(None::<std::time::Instant>));
        let weak = ui.weak();
        wheel.connect_scroll(move |_, _, dy| {
            if let Some(ui) = weak.upgrade()
                && ui.book_mode.is_active()
                && !ui.comparison.is_active()
            {
                if last
                    .get()
                    .is_none_or(|t| t.elapsed() > Duration::from_millis(350))
                    && dy.abs() > 0.01
                {
                    last.set(Some(std::time::Instant::now()));
                    ui.step(if dy > 0.0 { 1 } else { -1 });
                }
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        ui.book_surface.add_controller(wheel);
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = ui.weak();
        keys.connect_key_pressed(move |_, key, _, _| {
            if let Some(ui) = weak.upgrade()
                && ui.book_mode.is_active()
                && !ui.comparison.is_active()
                && (key == gdk::Key::Page_Down || key == gdk::Key::Page_Up)
            {
                ui.step(if key == gdk::Key::Page_Down { 1 } else { -1 });
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        ui.book_surface.add_controller(keys);
        ui.connect(app, &verse_menu);
        ui.relabel();
        ui.window.set_sensitive(false);
        ui.window.present();
        ui
    }
    fn weak(self: &Rc<Self>) -> Weak<Self> {
        Rc::downgrade(self)
    }
    fn t(&self, it: &str, en: &str) -> String {
        if self.state.borrow().language == "en" {
            en.into()
        } else {
            it.into()
        }
    }
    fn error(&self, error: impl ToString) {
        if !self.closed.get() {
            let error = error.to_string();
            eprintln!("Bibbia: {error}");
            self.toast.add_toast(adw::Toast::new(&error));
        }
    }
    fn launch(
        self: &Rc<Self>,
        future: impl std::future::Future<Output = Result<(), String>> + 'static,
    ) {
        let weak = self.weak();
        glib::MainContext::default().spawn_local(async move {
            if let Err(e) = future.await
                && let Some(ui) = weak.upgrade()
            {
                ui.error(e);
            }
        });
    }
    fn connect(self: &Rc<Self>, app: &adw::Application, verse_menu: &gtk::MenuButton) {
        let w = self.weak();
        self.window.connect_close_request(move |_| {
            if let Some(ui) = w.upgrade() {
                ui.close();
            }
            glib::Propagation::Proceed
        });
        let w = self.weak();
        self.book.connect_selected_notify(move |combo| {
            if let Some(ui) = w.upgrade()
                && ui.ready.get()
                && !ui.changing.get()
            {
                let mut p = ui.position();
                p.book = combo.selected() as usize;
                p.chapter = 0;
                p.verse = 0;
                let u = ui.clone();
                ui.launch(async move { u.navigate(p).await });
            }
        });
        let w = self.weak();
        self.chapter.connect_value_changed(move |spin| {
            if let Some(ui) = w.upgrade()
                && ui.ready.get()
                && !ui.changing.get()
            {
                let mut p = ui.position();
                p.chapter = spin.value_as_int().saturating_sub(1) as usize;
                p.verse = 0;
                let u = ui.clone();
                ui.launch(async move { u.navigate(p).await });
            }
        });
        let w = self.weak();
        self.edition.connect_selected_notify(move |combo| {
            if let Some(ui) = w.upgrade()
                && ui.ready.get()
                && !ui.changing.get()
            {
                let id = ui
                    .state
                    .borrow()
                    .editions
                    .get(combo.selected() as usize)
                    .map(|e| e.id.clone());
                if let Some(id) = id {
                    ui.choose_edition(id);
                }
            }
        });
        let w = self.weak();
        self.previous.connect_clicked(move |_| {
            if let Some(ui) = w.upgrade() {
                ui.step(-1)
            }
        });
        let w = self.weak();
        self.next.connect_clicked(move |_| {
            if let Some(ui) = w.upgrade() {
                ui.step(1)
            }
        });
        let w = self.weak();
        self.search_button.connect_clicked(move |_| {
            if let Some(ui) = w.upgrade() {
                let u = ui.clone();
                ui.launch(async move { u.search().await });
            }
        });
        let w = self.weak();
        self.query.connect_activate(move |_| {
            if let Some(ui) = w.upgrade() {
                ui.search_button.emit_clicked();
            }
        });
        let w = self.weak();
        self.stack.connect_visible_child_name_notify(move |stack| {
            if let Some(ui) = w.upgrade() {
                ui.popup.popdown();
                if stack.visible_child_name().as_deref() == Some("saved") && ui.ready.get() {
                    ui.saved_offset.set(0);
                    let u = ui.clone();
                    ui.launch(async move { u.load_saved().await });
                }
                if stack.visible_child_name().as_deref() == Some("search") {
                    ui.query.grab_focus();
                }
            }
        });
        for (view, store) in [
            (&self.search_view, &self.search_store),
            (&self.saved_view, &self.saved_store),
        ] {
            let w = self.weak();
            let store = store.clone();
            view.connect_activate(move |_, index| {
                if let (Some(ui), Some(row)) = (w.upgrade(), store.item(index)) {
                    let row = row.downcast::<glib::BoxedAnyObject>().unwrap();
                    let hit = row.borrow::<Hit>();
                    let mut p = ui.position();
                    p.book = hit.book;
                    p.chapter = hit.chapter;
                    p.verse = hit.verse;
                    ui.stack.set_visible_child_name("read");
                    let u = ui.clone();
                    ui.launch(async move { u.navigate(p).await });
                }
            });
        }
        for (button, forward) in [(&self.saved_previous, false), (&self.saved_next, true)] {
            let w = self.weak();
            button.connect_clicked(move |_| {
                if let Some(ui) = w.upgrade() {
                    ui.saved_offset.set(if forward {
                        ui.saved_offset.get() + 200
                    } else {
                        ui.saved_offset.get().saturating_sub(200)
                    });
                    let u = ui.clone();
                    ui.launch(async move { u.load_saved().await });
                }
            });
        }
        self.connect_annotations(verse_menu);
        for (name, keys) in [
            ("search", vec!["<Control>f"]),
            ("preferences", vec!["<Control>comma"]),
            ("quit", vec!["<Control>q"]),
            ("previous", vec!["<Alt>Left"]),
            ("next", vec!["<Alt>Right"]),
        ] {
            let action = gio::SimpleAction::new(name, None);
            let w = self.weak();
            action.connect_activate(move |_, _| {
                if let Some(ui) = w.upgrade() {
                    match name {
                        "search" => ui.stack.set_visible_child_name("search"),
                        "preferences" => ui.preferences(),
                        "quit" => ui.window.close(),
                        "previous" => ui.step(-1),
                        _ => ui.step(1),
                    }
                }
            });
            self.window.add_action(&action);
            app.set_accels_for_action(&format!("win.{name}"), &keys);
        }
        for name in ["import", "data", "about"] {
            let action = gio::SimpleAction::new(name, None);
            let w = self.weak();
            action.connect_activate(move |_, _| {
                if let Some(ui) = w.upgrade() {
                    match name {
                        "import" => ui.import(),
                        "data" => ui.open_data(),
                        _ => ui.about(),
                    }
                }
            });
            self.window.add_action(&action);
        }
    }
    pub async fn initialize(self: &Rc<Self>) -> Result<(), String> {
        let boot = self.core.call(json!({"op":"bootstrap"})).await?;
        if self.closed.get() {
            return Ok(());
        }
        {
            let mut s = self.state.borrow_mut();
            s.editions = serde_json::from_value(boot["editionMetadata"].clone())
                .map_err(|e| e.to_string())?;
            s.editions
                .sort_by(|a, b| b.catholic.cmp(&a.catholic).then(a.id.cmp(&b.id)));
            s.position.edition = boot["defaultEdition"].as_str().unwrap().into();
            let settings = &boot["settings"];
            if let Some(choice) = settings["comparisonEdition"].as_str() {
                self.comparison_choice.replace(choice.into());
            }
            if let Ok(p) = serde_json::from_value::<Position>(settings["position"].clone())
                && s.editions.iter().any(|e| e.id == p.edition)
            {
                s.position = p;
            }
            s.language = settings["language"].as_str().unwrap_or("it").into();
            s.filter = settings["bibleLanguage"].as_str().unwrap_or("it").into();
            s.font_size = settings["fontSize"]
                .as_f64()
                .unwrap_or(22.0)
                .clamp(14.0, 32.0);
            set_theme(settings["theme"].as_str().unwrap_or("Default"));
        }
        self.relabel();
        self.refresh_editions();
        self.ready.set(true);
        self.window.set_sensitive(true);
        self.navigate(self.position()).await
    }
    fn position(&self) -> Position {
        self.state.borrow().position.clone()
    }
    fn save_position(&self) {
        self.core
            .post(json!({"op":"settings","key":"position","value":self.position()}));
    }
    fn refresh_editions(&self) {
        self.changing.set(true);
        let s = self.state.borrow();
        let names: Vec<_> = s.editions.iter().map(|e| e.id.as_str()).collect();
        self.edition.set_model(Some(&gtk::StringList::new(&names)));
        self.edition.set_selected(
            s.editions
                .iter()
                .position(|e| e.id == s.position.edition)
                .unwrap_or(0) as u32,
        );
        self.changing.set(false);
    }
    fn choose_edition(self: &Rc<Self>, id: String) {
        if self.position().edition == id {
            return;
        }
        {
            let mut s = self.state.borrow_mut();
            let lang = s
                .editions
                .iter()
                .find(|e| e.id == id)
                .map(|e| e.language.clone())
                .unwrap_or("und".into());
            if s.filter != "all" && s.filter != lang {
                s.filter = if lang == "und" { "all".into() } else { lang };
                self.core
                    .post(json!({"op":"settings","key":"bibleLanguage","value":s.filter}));
            }
        }
        self.search_generation.set(self.search_generation.get() + 1);
        self.search_store.remove_all();
        self.saved_store.remove_all();
        let mut p = self.position();
        p.edition = id;
        p.book = 0;
        p.chapter = 0;
        p.verse = 0;
        let u = self.clone();
        self.launch(async move { u.navigate(p).await });
    }
    pub async fn navigate(self: &Rc<Self>, mut position: Position) -> Result<(), String> {
        self.pending_position.replace(Some(position.clone()));
        self.popup.popdown();
        self.note_popup.popdown();
        self.cancel_selection();
        self.generation.set(self.generation.get() + 1);
        let generation = self.generation.get();
        let cached = self.state.borrow().catalogs.get(&position.edition).cloned();
        let books = if let Some(b) = cached {
            b
        } else {
            let value = self
                .core
                .call(json!({"op":"books","edition":position.edition}))
                .await?;
            let books: Vec<Book> = serde_json::from_value(value).map_err(|e| e.to_string())?;
            Rc::new(books)
        };
        if self.closed.get() || generation != self.generation.get() {
            return Ok(());
        }
        position.book = position.book.min(books.len() - 1);
        position.chapter = position.chapter.min(books[position.book].chapters - 1);
        let value=self.core.call(json!({"op":"chapter","edition":position.edition,"book":position.book,"chapter":position.chapter})).await?;
        if self.closed.get() || generation != self.generation.get() {
            return Ok(());
        }
        let verses: Vec<Verse> = serde_json::from_value(value).map_err(|e| e.to_string())?;
        position.verse = position.verse.min(verses.len() - 1);
        let old = self.position();
        let changed = self.ready.get()
            && (old.edition != position.edition
                || old.book != position.book
                || old.chapter != position.chapter);
        let texture = if changed {
            self.book_surface.capture()
        } else {
            None
        };
        let capture_generation = self.book_surface.generation();
        let forward = old.edition != position.edition
            || (old.book, old.chapter) < (position.book, position.chapter);
        self.changing.set(true);
        {
            let mut s = self.state.borrow_mut();
            if !Rc::ptr_eq(&s.books, &books) {
                let names: Vec<_> = books.iter().map(|b| b.name.as_str()).collect();
                self.book.set_model(Some(&gtk::StringList::new(&names)));
            }
            s.catalogs.insert(position.edition.clone(), books.clone());
            s.books = books;
            s.position = position.clone();
            s.verses = verses;
            s.selection.clear();
            self.book.set_selected(position.book as u32);
            self.chapter
                .set_range(1.0, s.books[position.book].chapters as f64);
            self.chapter.set_value((position.chapter + 1) as f64);
            self.reference.set_text(&s.books[position.book].name);
            self.subtitle.set_text(&format!(
                "{} {}",
                if s.language == "en" {
                    "Chapter"
                } else {
                    "Capitolo"
                },
                position.chapter + 1
            ));
            self.previous
                .set_sensitive(position.book > 0 || position.chapter > 0);
            self.next.set_sensitive(
                position.book + 1 < s.books.len()
                    || position.chapter + 1 < s.books[position.book].chapters,
            );
        }
        self.changing.set(false);
        self.refresh_editions();
        self.book_seek.set(true);
        self.render();
        if changed {
            let surface = self.book_surface.clone();
            let weak = self.weak();
            glib::idle_add_local_once(move || {
                if let Some(ui) = weak.upgrade()
                    && !ui.closed.get()
                    && ui.generation.get() == generation
                    && surface.generation() == capture_generation
                {
                    surface.turn(texture, forward);
                }
            });
        }
        self.pending_position.borrow_mut().take();
        self.save_position();
        Ok(())
    }
    fn step(self: &Rc<Self>, direction: i32) {
        if self.turn_book(direction) {
            return;
        }
        if !self.ready.get() {
            return;
        }
        let mut p = self
            .pending_position
            .borrow()
            .clone()
            .unwrap_or_else(|| self.position());
        if p.edition != self.state.borrow().position.edition {
            return;
        }
        {
            let s = self.state.borrow();
            if direction < 0 {
                if p.chapter > 0 {
                    p.chapter -= 1;
                } else if p.book > 0 {
                    p.book -= 1;
                    p.chapter = s.books[p.book].chapters - 1;
                } else {
                    return;
                }
            } else if p.chapter + 1 < s.books[p.book].chapters {
                p.chapter += 1;
            } else if p.book + 1 < s.books.len() {
                p.book += 1;
                p.chapter = 0;
            } else {
                return;
            }
        }
        p.verse = 0;
        self.pending_position.replace(Some(p.clone()));
        let u = self.clone();
        self.launch(async move { u.navigate(p).await });
    }
    fn render(self: &Rc<Self>) {
        if self.comparison.is_active() {
            self.refresh_comparison();
            return;
        }
        self.changing.set(true);
        let (text, spans) = model::layout(&self.state.borrow().verses);
        self.reader.buffer().set_text(&text);
        self.state.borrow_mut().spans = spans;
        let buffer = self.reader.buffer();
        let (start, end) = buffer.bounds();
        buffer.apply_tag(&self.body_tag, &start, &end);
        self.body_tag
            .set_size_points(self.state.borrow().font_size * 0.75);
        self.update_gutter();
        for index in 0..self.state.borrow().verses.len() {
            self.style_verse(index);
        }
        let s = self.state.borrow();
        let span = &s.spans[s.position.verse];
        let iter = buffer.iter_at_offset(span.start);
        buffer.place_cursor(&iter);
        let mark = buffer.create_mark(None, &iter, true);
        let reader = self.reader.clone();
        let paged = self.book_mode.is_active();
        glib::idle_add_local_once(move || {
            if !paged {
                reader.scroll_to_mark(&mark, 0.08, true, 0.0, 0.12);
            }
            reader.buffer().delete_mark(&mark);
        });
        self.attribution
            .set_text(if s.position.edition == "CEI 2008" {
                CEI
            } else {
                ""
            });
        self.attribution
            .set_visible(s.position.edition == "CEI 2008");
        drop(s);
        self.changing.set(false);
        self.schedule_book();
        if self.comparison.is_active() {
            self.refresh_comparison();
        }
    }
    fn update_gutter(&self) {
        let s = self.state.borrow();
        let layout = self.reader.create_pango_layout(None);
        let mut font = pango::FontDescription::from_string("Adwaita Sans");
        font.set_size((s.font_size * 0.75 * 0.65 * f64::from(pango::SCALE)) as i32);
        layout.set_font_description(Some(&font));
        let width = s
            .verses
            .iter()
            .map(|v| {
                layout.set_text(&format!("★ {} ", v.label));
                layout.pixel_size().0
            })
            .max()
            .unwrap_or(24)
            + 8;
        let mut tabs = pango::TabArray::new(1, true);
        tabs.set_tab(0, pango::TabAlign::Left, width);
        self.reader.set_tabs(&tabs);
        self.reader.set_indent(-width);
        self.reader_right.set_tabs(&tabs);
        self.reader_right.set_indent(-width);
    }
    fn style_verse(&self, index: usize) {
        let s = self.state.borrow();
        let v = &s.verses[index];
        let span = &s.spans[index];
        let buffer = self.reader.buffer();
        let a = buffer.iter_at_offset(span.start);
        let b = buffer.iter_at_offset(span.end);
        let prefix = buffer.iter_at_offset(span.prefix);
        buffer.apply_tag(&self.number_tag, &prefix, &a);
        for name in ["underline", "yellow", "green", "blue", "pink"] {
            buffer.remove_tag_by_name(name, &a, &b);
        }
        if v.annotation.underline {
            buffer.apply_tag_by_name("underline", &a, &b);
        }
        if let Some(color) = &v.annotation.highlight {
            buffer.apply_tag_by_name(color, &a, &b);
        }
        for mark in &v.annotation.ranges {
            if let (Some(start), Some(end)) = (
                model::char_offset(&v.text, mark.start),
                model::char_offset(&v.text, mark.start + mark.length),
            ) {
                let from = buffer.iter_at_offset(span.start + start);
                let to = buffer.iter_at_offset(span.start + end);
                if mark.underline {
                    buffer.apply_tag_by_name("underline", &from, &to);
                }
                if let Some(color) = &mark.highlight {
                    buffer.apply_tag_by_name(color, &from, &to);
                }
            }
        }
    }
    fn relabel(&self) {
        self.relabel_annotations();
        self.relabel_book();
        self.comparison.set_label(&self.t("Confronta", "Compare"));
        self.comparison_edition
            .set_tooltip_text(Some(&self.t("Seconda traduzione", "Second translation")));
        self.header_title.set_subtitle(&self.position().edition);
        let en = self.state.borrow().language == "en";
        for (name, it, english) in [
            ("read", "Leggi", "Read"),
            ("search", "Cerca", "Search"),
            ("saved", "La mia raccolta", "My collection"),
        ] {
            if let Some(widget) = self.stack.child_by_name(name) {
                self.stack
                    .page(&widget)
                    .set_title(Some(if en { english } else { it }));
            }
        }
        self.book.set_tooltip_text(Some(&self.t("Libro", "Book")));
        self.chapter
            .set_tooltip_text(Some(&self.t("Capitolo", "Chapter")));
        self.edition
            .set_tooltip_text(Some(&self.t("Traduzione", "Translation")));
        self.saved_previous
            .set_tooltip_text(Some(&self.t("Pagina precedente", "Previous page")));
        self.saved_next
            .set_tooltip_text(Some(&self.t("Pagina successiva", "Next page")));
        self.query.set_placeholder_text(Some(if en {
            "A word or phrase…"
        } else {
            "Una parola o una frase…"
        }));
        self.search_button
            .set_label(if en { "Search" } else { "Cerca" });
        self.underline
            .set_tooltip_text(Some(if en { "Underline" } else { "Sottolinea" }));
        self.note_save
            .set_label(if en { "Save note" } else { "Salva nota" });
        self.search_status.set_text(&self.t(
            "Cerca nella traduzione selezionata. Fino a 200 risultati.",
            "Search the selected translation. Up to 200 results.",
        ));
        let menu = gio::Menu::new();
        for (it, english, action) in [
            ("Preferenze", "Preferences", "win.preferences"),
            ("Importa traduzione…", "Import translation…", "win.import"),
            ("Apri cartella dati", "Open data folder", "win.data"),
            ("Informazioni su Bibbia", "About Bibbia", "win.about"),
        ] {
            menu.append(Some(if en { english } else { it }), Some(action));
        }
        self.menu.set_menu_model(Some(&menu));
    }
    fn relabel_book(&self) {
        self.book_mode.set_label(&self.t("Libro", "Book"));
        self.book_mode
            .set_tooltip_text(Some(&self.t("Libro impaginato", "Paged book")));
        let (previous, next) = if self.book_mode.is_active() && !self.comparison.is_active() {
            (
                self.t("Pagina precedente", "Previous page"),
                self.t("Pagina successiva", "Next page"),
            )
        } else {
            (
                self.t("Capitolo precedente", "Previous chapter"),
                self.t("Capitolo successivo", "Next chapter"),
            )
        };
        self.previous.set_tooltip_text(Some(&previous));
        self.next.set_tooltip_text(Some(&next));
    }
    async fn search(self: &Rc<Self>) -> Result<(), String> {
        let query = self.query.text().to_string();
        self.search_generation.set(self.search_generation.get() + 1);
        let generation = self.search_generation.get();
        let p = self.position();
        self.search_store.remove_all();
        self.search_status
            .set_text(&self.t("Ricerca in corso…", "Searching…"));
        let result = self
            .core
            .call(json!({"op":"search","edition":p.edition,"query":query}))
            .await;
        if self.closed.get() || generation != self.search_generation.get() {
            return Ok(());
        }
        let result = match result {
            Ok(v) => v,
            Err(e) => {
                self.search_status.set_text(&e);
                return Ok(());
            }
        };
        let hits: Vec<Hit> =
            serde_json::from_value(result["hits"].clone()).map_err(|e| e.to_string())?;
        let count = hits.len();
        for hit in hits {
            self.search_store.append(&glib::BoxedAnyObject::new(hit));
        }
        self.search_status
            .set_text(&if result["truncated"] == true {
                self.t(
                    "Primi 200 risultati. Affina la ricerca.",
                    "First 200 results. Refine your search.",
                )
            } else {
                format!("{count} · {}", p.edition)
            });
        Ok(())
    }
    async fn load_saved(self: &Rc<Self>) -> Result<(), String> {
        let p = self.position();
        let offset = self.saved_offset.get();
        let value = self
            .core
            .call(json!({"op":"saved","edition":p.edition,"limit":200,"offset":offset}))
            .await?;
        if self.closed.get()
            || self.position().edition != p.edition
            || offset != self.saved_offset.get()
        {
            return Ok(());
        }
        let hits: Vec<Hit> = serde_json::from_value(value).map_err(|e| e.to_string())?;
        let count = hits.len();
        self.saved_store.remove_all();
        for hit in hits {
            self.saved_store.append(&glib::BoxedAnyObject::new(hit));
        }
        self.saved_status.set_text(&if count == 0 {
            self.t(
                "Nessun appunto in questa pagina.",
                "No annotations on this page.",
            )
        } else {
            format!("{}–{} · {}", offset + 1, offset + count, p.edition)
        });
        self.saved_previous.set_sensitive(offset > 0);
        self.saved_next.set_sensitive(count == 200);
        Ok(())
    }
    pub fn close(&self) {
        if self.closed.replace(true) {
            return;
        }
        self.cancel_selection();
        self.book_surface.cancel();
        if let Some(source) = self.font_source.borrow_mut().take() {
            source.remove();
        }
        if self.ready.get() {
            if !self.comparison.is_active() {
                self.save_book();
            }
            self.save_position();
            self.core.post(json!({"op":"settings","key":"fontSize","value":self.state.borrow().font_size as i64}));
        }
        self.core.shutdown();
        for popup in [&self.popup, &self.note_popup] {
            popup.popdown();
            if popup.parent().is_some() {
                popup.unparent();
            }
        }
    }
}
fn set_theme(value: &str) {
    adw::StyleManager::default().set_color_scheme(match value {
        "Dark" => adw::ColorScheme::ForceDark,
        "Light" => adw::ColorScheme::ForceLight,
        _ => adw::ColorScheme::Default,
    });
}
