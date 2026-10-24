//! Local Bible engine. The WinUI host exchanges UTF-8 JSON across a small C ABI.
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    ffi::{CStr, CString, c_char},
    path::Path,
    sync::{Mutex, OnceLock},
};

type Result<T> = std::result::Result<T, String>;
const KJV: &str = include_str!("../data/kjv.json");
const DIODATI: &str = include_str!("../data/diodati.json");
const DEFAULT_EDITION: &str = "Diodati 1649";

#[derive(Clone, Deserialize, Serialize)]
pub struct Book {
    pub name: String,
    pub abbrev: String,
    pub chapters: Vec<Vec<String>>,
}

#[derive(Clone, Default, Deserialize, Serialize, PartialEq, Debug)]
pub struct Annotation {
    pub highlight: Option<String>,
    pub underline: bool,
    pub bookmark: bool,
    pub note: String,
}

pub struct Engine {
    db: Connection,
    editions: BTreeMap<String, Vec<Book>>,
}

fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn field<'a>(v: &'a Value, name: &str) -> Result<&'a str> {
    v.get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Campo mancante: {name}"))
}
fn index(v: &Value, name: &str) -> Result<usize> {
    v.get(name)
        .and_then(Value::as_u64)
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| format!("Indice non valido: {name}"))
}

impl Engine {
    pub fn open(directory: &Path) -> Result<Self> {
        std::fs::create_dir_all(directory).map_err(error)?;
        let db = Connection::open(directory.join("bible.sqlite3")).map_err(error)?;
        db.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;
            CREATE TABLE IF NOT EXISTS annotations (
                edition TEXT NOT NULL, book INTEGER NOT NULL, chapter INTEGER NOT NULL,
                verse INTEGER NOT NULL, payload TEXT NOT NULL,
                PRIMARY KEY (edition, book, chapter, verse));
            CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, payload TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS editions (id TEXT PRIMARY KEY, payload TEXT NOT NULL);",
        )
        .map_err(error)?;
        let mut editions = BTreeMap::new();
        editions.insert("KJV".into(), serde_json::from_str(KJV).map_err(error)?);
        editions.insert(
            DEFAULT_EDITION.into(),
            serde_json::from_str(DIODATI).map_err(error)?,
        );
        {
            let mut stmt = db
                .prepare("SELECT id, payload FROM editions")
                .map_err(error)?;
            let rows = stmt
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                .map_err(error)?;
            for row in rows {
                let (id, payload) = row.map_err(error)?;
                editions.insert(id, serde_json::from_str(&payload).map_err(error)?);
            }
        }
        Ok(Self { db, editions })
    }

    fn books(&self, edition: &str) -> Result<&Vec<Book>> {
        self.editions
            .get(edition)
            .ok_or_else(|| "Traduzione non disponibile".into())
    }
    fn chapter(&self, edition: &str, book: usize, chapter: usize) -> Result<&Vec<String>> {
        self.books(edition)?
            .get(book)
            .and_then(|b| b.chapters.get(chapter))
            .ok_or_else(|| "Capitolo non disponibile".into())
    }
    fn annotation(
        &self,
        edition: &str,
        book: usize,
        chapter: usize,
        verse: usize,
    ) -> Result<Annotation> {
        let mut stmt = self.db.prepare("SELECT payload FROM annotations WHERE edition=? AND book=? AND chapter=? AND verse=?").map_err(error)?;
        let mut rows = stmt
            .query(params![edition, book as i64, chapter as i64, verse as i64])
            .map_err(error)?;
        match rows.next().map_err(error)? {
            Some(row) => {
                serde_json::from_str(&row.get::<_, String>(0).map_err(error)?).map_err(error)
            }
            None => Ok(Annotation::default()),
        }
    }

    pub fn request(&mut self, request: Value) -> Result<Value> {
        let op = field(&request, "op")?;
        let edition = request
            .get("edition")
            .and_then(Value::as_str)
            .unwrap_or("KJV");
        match op {
            "bootstrap" => {
                let mut settings = serde_json::Map::new();
                let mut stmt = self
                    .db
                    .prepare("SELECT key, payload FROM settings")
                    .map_err(error)?;
                let rows = stmt
                    .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                    .map_err(error)?;
                for row in rows {
                    let (k, v) = row.map_err(error)?;
                    settings.insert(k, serde_json::from_str(&v).map_err(error)?);
                }
                Ok(
                    json!({"editions":self.editions.keys().collect::<Vec<_>>(), "defaultEdition":DEFAULT_EDITION, "settings":settings}),
                )
            }
            "books" => Ok(json!(
                self.books(edition)?
                    .iter()
                    .enumerate()
                    .map(|(i, b)| json!({"index":i,"name":b.name,"chapters":b.chapters.len()}))
                    .collect::<Vec<_>>()
            )),
            "chapter" => {
                let book = index(&request, "book")?;
                let chapter = index(&request, "chapter")?;
                let verses = self.chapter(edition, book, chapter)?;
                let mut result = Vec::with_capacity(verses.len());
                for (i, text) in verses.iter().enumerate() {
                    result.push(json!({"verse":i,"text":text,"annotation":self.annotation(edition,book,chapter,i)?}));
                }
                Ok(json!(result))
            }
            "annotate" => {
                let book = index(&request, "book")?;
                let chapter = index(&request, "chapter")?;
                let verse = index(&request, "verse")?;
                if verse >= self.chapter(edition, book, chapter)?.len() {
                    return Err("Versetto non disponibile".into());
                }
                let annotation: Annotation = serde_json::from_value(
                    request
                        .get("annotation")
                        .cloned()
                        .ok_or("Annotazione mancante")?,
                )
                .map_err(error)?;
                if let Some(color) = &annotation.highlight
                    && !["yellow", "green", "blue", "pink"].contains(&color.as_str())
                {
                    return Err("Colore non valido".into());
                }
                if annotation.note.len() > 32_768 {
                    return Err("Nota troppo lunga".into());
                }
                if annotation == Annotation::default() {
                    self.db.execute("DELETE FROM annotations WHERE edition=? AND book=? AND chapter=? AND verse=?",
                        params![edition,book as i64,chapter as i64,verse as i64]).map_err(error)?;
                } else {
                    self.db
                        .execute(
                            "INSERT OR REPLACE INTO annotations VALUES (?,?,?,?,?)",
                            params![
                                edition,
                                book as i64,
                                chapter as i64,
                                verse as i64,
                                serde_json::to_string(&annotation).map_err(error)?
                            ],
                        )
                        .map_err(error)?;
                }
                Ok(json!(annotation))
            }
            "saved" => {
                self.books(edition)?;
                let mut stmt = self.db.prepare("SELECT book, chapter, verse, payload FROM annotations WHERE edition=? ORDER BY book,chapter,verse").map_err(error)?;
                let rows = stmt
                    .query_map([edition], |r| {
                        Ok((
                            r.get::<_, u32>(0)?,
                            r.get::<_, u32>(1)?,
                            r.get::<_, u32>(2)?,
                            r.get::<_, String>(3)?,
                        ))
                    })
                    .map_err(error)?;
                let mut result = Vec::new();
                for row in rows {
                    let (b, c, v, a) = row.map_err(error)?;
                    let (b, c, v) = (b as usize, c as usize, v as usize);
                    let annotation: Annotation = serde_json::from_str(&a).map_err(error)?;
                    let name = &self
                        .books(edition)?
                        .get(b)
                        .ok_or("Libro salvato non valido")?
                        .name;
                    let text = self
                        .chapter(edition, b, c)?
                        .get(v)
                        .ok_or("Versetto salvato non valido")?;
                    result.push(json!({"book":b,"chapter":c,"verse":v,"name":name,
                        "text":text,"annotation":annotation}));
                }
                Ok(json!(result))
            }
            "search" => {
                let query = field(&request, "query")?.trim().to_lowercase();
                if query.chars().count() < 2 {
                    return Err("Inserisci almeno due caratteri".into());
                }
                if query.len() > 256 {
                    return Err("Ricerca troppo lunga".into());
                }
                let mut hits = Vec::new();
                let mut truncated = false;
                'outer: for (bi, b) in self.books(edition)?.iter().enumerate() {
                    for (ci, c) in b.chapters.iter().enumerate() {
                        for (vi, text) in c.iter().enumerate() {
                            if text.to_lowercase().contains(&query) {
                                if hits.len() == 200 {
                                    truncated = true;
                                    break 'outer;
                                }
                                hits.push(json!({"book":bi,"chapter":ci,"verse":vi,"name":b.name,"text":text}));
                            }
                        }
                    }
                }
                Ok(json!({"hits":hits,"truncated":truncated}))
            }
            "settings" => {
                let key = field(&request, "key")?;
                if !["position", "fontSize", "theme"].contains(&key) {
                    return Err("Impostazione sconosciuta".into());
                }
                let value = request.get("value").ok_or("Valore mancante")?;
                if key == "fontSize" && !value.as_u64().is_some_and(|n| (14..=32).contains(&n)) {
                    return Err("Dimensione non valida".into());
                }
                if key == "theme"
                    && !value
                        .as_str()
                        .is_some_and(|s| ["Default", "Light", "Dark"].contains(&s))
                {
                    return Err("Tema non valido".into());
                }
                if key == "position" {
                    let e = field(value, "edition")?;
                    let verses =
                        self.chapter(e, index(value, "book")?, index(value, "chapter")?)?;
                    if let Some(verse) = value.get("verse")
                        && !verse.as_u64().is_some_and(|v| v < verses.len() as u64)
                    {
                        return Err("Versetto non disponibile".into());
                    }
                }
                self.db
                    .execute(
                        "INSERT OR REPLACE INTO settings VALUES (?,?)",
                        params![key, value.to_string()],
                    )
                    .map_err(error)?;
                Ok(json!(true))
            }
            "import" => {
                let id = field(&request, "id")?.trim();
                if id.is_empty() || id.len() > 80 || ["KJV", DEFAULT_EDITION].contains(&id) {
                    return Err("Nome traduzione non valido o riservato".into());
                }
                if self.editions.contains_key(id) {
                    return Err("Traduzione già presente: scegli un nome diverso".into());
                }
                let path = Path::new(field(&request, "path")?);
                if std::fs::metadata(path).map_err(error)?.len() > 32 * 1024 * 1024 {
                    return Err("File troppo grande (massimo 32 MB)".into());
                }
                let source = std::fs::read_to_string(path).map_err(error)?;
                let books: Vec<Book> =
                    serde_json::from_str(source.trim_start_matches('\u{feff}')).map_err(error)?;
                if books.is_empty()
                    || books.len() > 100
                    || books.iter().any(|b| {
                        b.name.trim().is_empty()
                            || b.abbrev.is_empty()
                            || b.chapters.is_empty()
                            || b.chapters.len() > 200
                            || b.chapters.iter().any(|c| {
                                c.is_empty()
                                    || c.len() > 200
                                    || c.iter().any(|v| v.trim().is_empty())
                            })
                    })
                {
                    return Err("Struttura Bibbia non valida".into());
                }
                let payload = serde_json::to_string(&books).map_err(error)?;
                self.db
                    .execute("INSERT INTO editions VALUES (?,?)", params![id, payload])
                    .map_err(error)?;
                self.editions.insert(id.to_string(), books);
                Ok(json!({"id":id}))
            }
            _ => Err("Operazione sconosciuta".into()),
        }
    }
}

static ENGINE: OnceLock<Mutex<Option<Engine>>> = OnceLock::new();
fn dispatch(request: Value) -> Result<Value> {
    let mut slot = ENGINE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Motore non disponibile")?;
    if request.get("op").and_then(Value::as_str) == Some("close") {
        *slot = None;
        return Ok(json!(true));
    }
    if request.get("op").and_then(Value::as_str) == Some("init") {
        *slot = Some(Engine::open(Path::new(field(&request, "directory")?))?);
        return slot.as_mut().unwrap().request(json!({"op":"bootstrap"}));
    }
    slot.as_mut()
        .ok_or("Motore non inizializzato")?
        .request(request)
}

/// # Safety
/// `request` must point to a NUL-terminated UTF-8 string, valid for this call.
/// Free the returned allocation exactly once with `bible_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bible_request(request: *const c_char) -> *mut c_char {
    let outcome = std::panic::catch_unwind(|| {
        if request.is_null() {
            return Err("Richiesta nulla".to_string());
        }
        let input = unsafe { CStr::from_ptr(request) }.to_str().map_err(error)?;
        dispatch(serde_json::from_str(input).map_err(error)?)
    })
    .unwrap_or_else(|_| Err("Errore interno del motore".into()));
    let response = match outcome {
        Ok(data) => json!({"ok":true,"data":data}),
        Err(e) => json!({"ok":false,"error":e}),
    };
    CString::new(response.to_string())
        .expect("JSON escapes NUL")
        .into_raw()
}

/// # Safety
/// `response` must be null or a live pointer returned by `bible_request`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bible_free(response: *mut c_char) {
    if !response.is_null() {
        drop(unsafe { CString::from_raw(response) });
    }
}

#[cfg(test)]
mod tests;
