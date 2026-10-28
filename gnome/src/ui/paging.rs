use super::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(super) struct ReaderPlace {
    pub book_mode: bool,
    pub edition: String,
    pub book: usize,
    pub chapter: usize,
    pub verse: usize,
    pub offset: usize,
}
impl Default for ReaderPlace {
    fn default() -> Self {
        Self {
            book_mode: true,
            edition: String::new(),
            book: 0,
            chapter: 0,
            verse: 0,
            offset: 0,
        }
    }
}
#[derive(Clone)]
pub(super) struct Page {
    pub y: i32,
    pub character: i32,
}
#[derive(Default)]
pub(super) struct Pager {
    pub pages: Vec<Page>,
    pub index: usize,
    pub columns: usize,
    pub anchor: i32,
    pub chapter: String,
    pub last: bool,
    pub pending: bool,
    pub applying: bool,
    pub place: ReaderPlace,
    pub directory: PathBuf,
}
impl Ui {
    pub(super) fn schedule_book(self: &Rc<Self>) {
        if self.pager.borrow().pending || self.closed.get() {
            return;
        }
        self.pager.borrow_mut().pending = true;
        let weak = self.weak();
        glib::idle_add_local_once(move || {
            if let Some(ui) = weak.upgrade() {
                ui.pager.borrow_mut().pending = false;
                ui.reflow_book();
            }
        });
    }
    pub(super) fn set_book_mode(self: &Rc<Self>, value: bool) {
        self.book_mode.set_active(value);
        self.book_surface.cancel();
        self.schedule_book();
        self.save_book();
    }
    pub(super) fn reflow_book(self: &Rc<Self>) {
        if !self.ready.get() || self.pager.borrow().applying {
            return;
        }
        let width = self.book_surface.width();
        let height = self.book_surface.height();
        if width < 80 || height < 80 {
            return;
        }
        let mode = self.book_mode.is_active();
        let columns = if mode && width >= 1040 { 2 } else { 1 };
        let w = (width - (columns - 1) * 24) / columns;
        for (n, (reader, scrolled)) in [
            (&self.reader, &self.read_scroll),
            (&self.reader_right, &self.right_scroll),
        ]
        .iter()
        .enumerate()
        {
            reader.set_bottom_margin(if mode { height } else { 24 });
            scrolled.set_policy(
                gtk::PolicyType::Never,
                if mode {
                    gtk::PolicyType::External
                } else {
                    gtk::PolicyType::Automatic
                },
            );
            scrolled.set_size_request(w, height);
            self.read_panes
                .move_(*scrolled, n as f64 * f64::from(w + 24), 0.0);
            scrolled.set_visible(n < (columns as usize));
        }
        if self.read_scroll.width() != w {
            let weak = self.weak();
            glib::timeout_add_local_once(Duration::from_millis(40), move || {
                if let Some(ui) = weak.upgrade() {
                    ui.schedule_book();
                }
            });
            return;
        }
        if !mode {
            self.page_counter.set_text("");
            return;
        }
        let mut lines = Vec::new();
        let mut iter = self.reader.buffer().start_iter();
        loop {
            if iter.is_end() {
                break;
            }
            let rect = self.reader.iter_location(&iter);
            lines.push((rect.y(), rect.height(), iter.offset()));
            if !self.reader.forward_display_line(&mut iter) {
                break;
            }
        }
        let mut pages = vec![Page { y: 0, character: 0 }];
        let mut start = 0;
        for (y, h, character) in lines {
            if y + h > start + height - 6 && y > start + 1 {
                pages.push(Page { y, character });
                start = y;
            }
        }
        let position = self.position();
        let key = format!(
            "{}/{}/{}",
            position.edition, position.book, position.chapter
        );
        let s = self.state.borrow();
        let mut pager = self.pager.borrow_mut();
        let seek = self.book_seek.replace(false);
        if pager.chapter != key || seek {
            pager.anchor = s.spans.get(position.verse).map_or(0, |v| v.start);
            if pager.chapter.is_empty()
                && pager.place.edition == position.edition
                && pager.place.book == position.book
                && pager.place.chapter == position.chapter
                && pager.place.verse == position.verse
                && let Some(v) = s.verses.get(position.verse)
            {
                pager.anchor += model::char_offset(&v.text, pager.place.offset).unwrap_or(0);
            }
            pager.chapter = key;
        }
        pager.columns = columns as usize;
        pager.pages = pages;
        pager.index = pager
            .pages
            .iter()
            .rposition(|p| p.character <= pager.anchor)
            .unwrap_or(0)
            / pager.columns
            * pager.columns;
        if pager.last {
            pager.index = (pager.pages.len() - 1) / pager.columns * pager.columns;
            pager.last = false;
        }
        drop(pager);
        drop(s);
        self.show_book();
    }
    pub(super) fn show_book(&self) {
        let mut p = self.pager.borrow_mut();
        if p.pages.is_empty() {
            return;
        }
        p.applying = true;
        let height = self.book_surface.height();
        for (n, scrolled) in [&self.read_scroll, &self.right_scroll].iter().enumerate() {
            let i = p.index + n;
            scrolled.set_visible(n < p.columns && i < p.pages.len());
            if i >= p.pages.len() {
                continue;
            }
            let h = if i + 1 < p.pages.len() {
                height.min(p.pages[i + 1].y - p.pages[i].y)
            } else {
                height
            };
            scrolled.set_height_request(h.max(1));
            scrolled.vadjustment().set_value(f64::from(p.pages[i].y));
        }
        p.anchor = p.pages[p.index].character;
        let range = if p.columns == 2 && p.index + 1 < p.pages.len() {
            format!("{}–{}", p.index + 1, p.index + 2)
        } else {
            (p.index + 1).to_string()
        };
        self.page_counter
            .set_text(&format!("{range} / {}", p.pages.len()));
        p.applying = false;
        self.previous
            .set_sensitive(p.index > 0 || self.position().book > 0 || self.position().chapter > 0);
        let s = self.state.borrow();
        self.next.set_sensitive(
            p.index + p.columns < p.pages.len()
                || s.position.book + 1 < s.books.len()
                || s.position.chapter + 1 < s.books[s.position.book].chapters,
        );
    }
    pub(super) fn turn_book(self: &Rc<Self>, direction: i32) -> bool {
        if !self.book_mode.is_active() {
            return false;
        }
        let mut p = self.pager.borrow_mut();
        if p.pages.is_empty() {
            return false;
        }
        let target = p.index as i32 + direction * p.columns as i32;
        if target < 0 || target >= p.pages.len() as i32 {
            p.last = direction < 0;
            return false;
        }
        drop(p);
        self.popup.popdown();
        self.note_popup.popdown();
        self.cancel_selection();
        let texture = self.book_surface.capture();
        self.pager.borrow_mut().index = target as usize;
        self.show_book();
        self.save_book();
        self.book_surface.turn(texture, direction > 0);
        true
    }
    pub(super) fn save_book(&self) {
        let s = self.state.borrow();
        let mut p = self.pager.borrow_mut();
        let verse = s
            .spans
            .iter()
            .position(|span| span.end > p.anchor)
            .unwrap_or_else(|| s.spans.len().saturating_sub(1));
        let offset = s
            .spans
            .get(verse)
            .zip(s.verses.get(verse))
            .map_or(0, |(span, v)| {
                model::utf16_offset(&v.text, (p.anchor - span.start).max(0) as usize)
            });
        p.place = ReaderPlace {
            book_mode: self.book_mode.is_active(),
            edition: s.position.edition.clone(),
            book: s.position.book,
            chapter: s.position.chapter,
            verse,
            offset,
        };
        if let Ok(data) = serde_json::to_vec(&p.place) {
            let path = p.directory.join("reader.json");
            let temp = p.directory.join("reader.json.tmp");
            if std::fs::write(&temp, data).is_ok() {
                let _ = std::fs::rename(temp, path);
            }
        }
        drop(p);
        drop(s);
        if self.book_mode.is_active() {
            self.state.borrow_mut().position.verse = verse;
            self.save_position();
        }
    }
}
