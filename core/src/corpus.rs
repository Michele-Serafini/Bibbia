//! Read-only, build-time text indexes. Reading a chapter never parses a Bible.
use crate::{Book, Result, error};
use rusqlite::Connection;
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize)]
pub struct Span {
    pub first: usize,
    pub count: usize,
    pub labels: Vec<String>,
}
#[derive(Deserialize)]
pub struct Header {
    pub name: String,
    pub abbrev: String,
    pub chapters: Vec<Span>,
}
pub struct Builtin {
    pub id: &'static str,
    pub book_count: usize,
    pub language: &'static str,
    pub canon: &'static str,
    catalog_json: &'static str,
    catalog: OnceLock<Vec<Header>>,
    text: &'static str,
    lower: &'static str,
    offsets: &'static [u8],
    lower_offsets: &'static [u8],
}
include!(concat!(env!("OUT_DIR"), "/corpora.rs"));
impl Builtin {
    pub fn catalog(&self) -> &[Header] {
        self.catalog.get_or_init(|| {
            serde_json::from_str(self.catalog_json).expect("validated build-time catalog")
        })
    }
    fn slice(text: &'static str, offsets: &[u8], index: usize) -> &'static str {
        let at =
            |i: usize| u32::from_le_bytes(offsets[i * 4..i * 4 + 4].try_into().unwrap()) as usize;
        &text[at(index)..at(index + 1)]
    }
    pub fn text(&self, index: usize) -> &'static str {
        Self::slice(self.text, self.offsets, index)
    }
    pub fn lower(&self, index: usize) -> &'static str {
        Self::slice(self.lower, self.lower_offsets, index)
    }
}
pub struct Edition {
    pub builtin: Option<&'static Builtin>,
    pub language: String,
    pub canon: String,
    pub book_count: usize,
    pub loaded: OnceLock<Vec<Book>>,
    pub lowercase: OnceLock<Vec<Vec<Vec<String>>>>,
}
impl Edition {
    pub fn builtin(source: &'static Builtin) -> Self {
        Self {
            builtin: Some(source),
            language: source.language.into(),
            canon: source.canon.into(),
            book_count: source.book_count,
            loaded: OnceLock::new(),
            lowercase: OnceLock::new(),
        }
    }
    pub fn imported(books: usize) -> Self {
        Self {
            builtin: None,
            language: "und".into(),
            canon: "custom".into(),
            book_count: books,
            loaded: OnceLock::new(),
            lowercase: OnceLock::new(),
        }
    }
    pub fn books<'a>(&'a self, db: &Connection, id: &str) -> Result<&'a [Book]> {
        if self.loaded.get().is_none() {
            let payload: String = db
                .query_row("SELECT payload FROM editions WHERE id=?", [id], |r| {
                    r.get(0)
                })
                .map_err(error)?;
            let data = serde_json::from_str(&payload).map_err(error)?;
            let _ = self.loaded.set(data);
        }
        Ok(self.loaded.get().unwrap())
    }
    pub fn release(&mut self) {
        self.loaded.take();
        self.lowercase.take();
    }
}
pub struct BookRef<'a> {
    pub name: &'a str,
    pub abbrev: &'a str,
    pub chapters: usize,
}
pub enum Chapter<'a> {
    Packed(&'static Builtin, &'static Span),
    Owned(&'a [String], Option<&'a [String]>),
}
impl<'a> Chapter<'a> {
    pub fn len(&self) -> usize {
        match self {
            Self::Packed(_, s) => s.count,
            Self::Owned(v, _) => v.len(),
        }
    }
    pub fn get(&self, index: usize) -> Option<&'a str> {
        if index >= self.len() {
            return None;
        }
        Some(match self {
            Self::Packed(b, s) => b.text(s.first + index),
            Self::Owned(v, _) => &v[index],
        })
    }
    pub fn label(&self, index: usize) -> String {
        let labels = match self {
            Self::Packed(_, s) => Some(s.labels.as_slice()),
            Self::Owned(_, l) => *l,
        };
        labels
            .and_then(|l| l.get(index))
            .cloned()
            .unwrap_or_else(|| (index + 1).to_string())
    }
}
