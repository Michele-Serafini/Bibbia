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
mod calendar;
mod comparison;
mod corpus;
mod organization;
mod reading;
mod selection;
mod transfer;
use corpus::{BookRef, Chapter, Edition};
use selection::TextMark;
#[cfg(test)]
const KJV: &str = include_str!("../data/kjv.json");
#[cfg(test)]
const DIODATI: &str = include_str!("../data/diodati.json");
#[cfg(test)]
const MARTINI: &str = include_str!("../data/martini.json");
const DEFAULT_EDITION: &str = "Martini 1781 (cattolica)";

#[derive(Clone, Deserialize, Serialize)]
pub struct Book {
    pub name: String,
    pub abbrev: String,
    pub chapters: Vec<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<Vec<String>>>,
}

#[derive(Clone, Default, Deserialize, Serialize, PartialEq, Debug)]
pub struct Annotation {
    pub highlight: Option<String>,
    pub underline: bool,
    pub bookmark: bool,
    pub note: String,
    #[serde(default)]
    pub ranges: Vec<TextMark>,
    #[serde(default, rename = "noteColor", skip_serializing_if = "Option::is_none")]
    pub note_color: Option<String>,
    #[serde(default, rename = "noteStyle", skip_serializing_if = "Option::is_none")]
    pub note_style: Option<String>,
}

pub struct Engine {
    directory: std::path::PathBuf,
    db: Connection,
    editions: BTreeMap<String, Edition>,
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
        let mut db = Connection::open(directory.join("bible.sqlite3")).map_err(error)?;
        transfer::apply_pending(&mut db, directory)?;
        db.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA cache_size=-1024;
            CREATE TABLE IF NOT EXISTS annotations (
                edition TEXT NOT NULL, book INTEGER NOT NULL, chapter INTEGER NOT NULL,
                verse INTEGER NOT NULL, payload TEXT NOT NULL,
                PRIMARY KEY (edition, book, chapter, verse));
            CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, payload TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS editions (id TEXT PRIMARY KEY, payload TEXT NOT NULL, book_count INTEGER);",
        )
        .map_err(error)?;
        db.execute_batch(organization::SCHEMA).map_err(error)?;
        db.execute_batch(reading::SCHEMA).map_err(error)?;
        // Upgrade old installations once, without parsing imported corpora on
        // subsequent launches. Personal annotations and source text are untouched.
        let has_count = db
            .prepare("PRAGMA table_info(editions)")
            .map_err(error)?
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?
            .iter()
            .any(|n| n == "book_count");
        if !has_count {
            db.execute_batch("ALTER TABLE editions ADD COLUMN book_count INTEGER;")
                .map_err(error)?;
        }
        db.execute(
            "UPDATE editions SET book_count=json_array_length(payload) WHERE book_count IS NULL",
            [],
        )
        .map_err(error)?;
        let mut editions = BTreeMap::new();
        for source in &corpus::BUILTINS {
            editions.insert(source.id.into(), Edition::builtin(source));
        }
        {
            // Only the catalog is needed at startup. Imported texts stay on
            // disk until opened; comparison temporarily retains its two inputs.
            let mut stmt = db
                .prepare("SELECT id, book_count FROM editions")
                .map_err(error)?;
            let rows = stmt
                .query_map([], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)? as usize))
                })
                .map_err(error)?;
            let imported = rows
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(error)?;
            for (mut id, count) in imported {
                // A previously imported CEI (or another new builtin name) must
                // keep its own text, notes and reading position after upgrade.
                if editions.contains_key(&id) {
                    let original = id.clone();
                    let mut suffix = 1;
                    loop {
                        id = if suffix == 1 {
                            format!("{original} (importata)")
                        } else {
                            format!("{original} (importata {suffix})")
                        };
                        let occupied: bool = db.query_row(
                            "SELECT EXISTS(SELECT 1 FROM editions WHERE id=?1 UNION ALL SELECT 1 FROM annotations WHERE edition=?1)",
                            [&id], |r| r.get(0)).map_err(error)?;
                        if !occupied && !editions.contains_key(&id) {
                            break;
                        }
                        suffix += 1;
                    }
                    let tx = db.unchecked_transaction().map_err(error)?;
                    tx.execute(
                        "UPDATE editions SET id=?1 WHERE id=?2",
                        params![id, original],
                    )
                    .map_err(error)?;
                    tx.execute(
                        "UPDATE annotations SET edition=?1 WHERE edition=?2",
                        params![id, original],
                    )
                    .map_err(error)?;
                    tx.execute("UPDATE reading_plans SET payload=json_set(payload,'$.edition',?1,'$.resume.edition',?1) WHERE json_extract(payload,'$.edition')=?2",params![id,original]).map_err(error)?;
                    tx.execute(
                        "UPDATE note_organization SET edition=?1 WHERE edition=?2",
                        params![id, original],
                    )
                    .map_err(error)?;
                    tx.execute("UPDATE settings SET payload=json_set(payload,'$.edition',?1) WHERE key='position' AND json_extract(payload,'$.edition')=?2",params![id,original]).map_err(error)?;
                    tx.commit().map_err(error)?;
                }
                editions.insert(id, Edition::imported(count));
            }
        }
        Ok(Self {
            directory: directory.into(),
            db,
            editions,
        })
    }

    fn save_annotation(
        &self,
        edition: &str,
        book: usize,
        chapter: usize,
        verse: usize,
        a: &Annotation,
    ) -> Result<()> {
        if a.highlight.is_none()
            && !a.underline
            && !a.bookmark
            && a.note.is_empty()
            && a.ranges.is_empty()
        {
            self.db.execute("DELETE FROM note_organization WHERE edition=? AND book=? AND chapter=? AND verse=?",params![edition,book as i64,chapter as i64,verse as i64]).map_err(error)?;
            self.db
                .execute(
                    "DELETE FROM annotations WHERE edition=? AND book=? AND chapter=? AND verse=?",
                    params![edition, book as i64, chapter as i64, verse as i64],
                )
                .map_err(error)?;
        } else {
            self.db
                .execute(
                    "INSERT OR REPLACE INTO annotations VALUES (?,?,?,?,?)",
                    params![
                        edition,
                        book as i64,
                        chapter as i64,
                        verse as i64,
                        serde_json::to_string(a).map_err(error)?
                    ],
                )
                .map_err(error)?;
        }
        Ok(())
    }

    fn edition(&self, id: &str) -> Result<&Edition> {
        self.editions
            .get(id)
            .ok_or_else(|| "Traduzione non disponibile".into())
    }
    fn books(&self, id: &str) -> Result<Vec<BookRef<'_>>> {
        let edition = self.edition(id)?;
        Ok(if let Some(source) = edition.builtin {
            source
                .catalog()
                .iter()
                .map(|b| BookRef {
                    name: &b.name,
                    abbrev: &b.abbrev,
                    chapters: b.chapters.len(),
                })
                .collect()
        } else {
            edition
                .books(&self.db, id)?
                .iter()
                .map(|b| BookRef {
                    name: &b.name,
                    abbrev: &b.abbrev,
                    chapters: b.chapters.len(),
                })
                .collect()
        })
    }
    fn chapter(&self, id: &str, book: usize, chapter: usize) -> Result<Chapter<'_>> {
        let edition = self.edition(id)?;
        if let Some(source) = edition.builtin {
            let span = source
                .catalog()
                .get(book)
                .and_then(|b| b.chapters.get(chapter))
                .ok_or("Capitolo non disponibile")?;
            Ok(Chapter::Packed(source, span))
        } else {
            let book = edition
                .books(&self.db, id)?
                .get(book)
                .ok_or("Capitolo non disponibile")?;
            let verses = book
                .chapters
                .get(chapter)
                .ok_or("Capitolo non disponibile")?;
            Ok(Chapter::Owned(
                verses,
                book.labels
                    .as_ref()
                    .and_then(|l| l.get(chapter))
                    .map(Vec::as_slice),
            ))
        }
    }
    fn search(&self, id: &str, query: &str) -> Result<Value> {
        let edition = self.edition(id)?;
        let mut hits = Vec::new();
        let mut truncated = false;
        let mut add = |bi, ci, vi, name: &str, text: &str, label: Option<&str>| {
            if hits.len() == 200 {
                truncated = true;
                return false;
            }
            hits.push(json!({"book":bi,"chapter":ci,"verse":vi,"name":name,"text":text,"label":label.map(str::to_owned).unwrap_or_else(||format!("{}",vi+1))}));
            true
        };
        if let Some(source) = edition.builtin {
            'outer: for (bi, b) in source.catalog().iter().enumerate() {
                for (ci, c) in b.chapters.iter().enumerate() {
                    for vi in 0..c.count {
                        if source.lower(c.first + vi).contains(query)
                            && !add(
                                bi,
                                ci,
                                vi,
                                &b.name,
                                source.text(c.first + vi),
                                c.labels.get(vi).map(String::as_str),
                            )
                        {
                            break 'outer;
                        }
                    }
                }
            }
        } else {
            let books = edition.books(&self.db, id)?;
            'outer: for (bi, b) in books.iter().enumerate() {
                for (ci, c) in b.chapters.iter().enumerate() {
                    for (vi, text) in c.iter().enumerate() {
                        if text.to_lowercase().contains(query)
                            && !add(
                                bi,
                                ci,
                                vi,
                                &b.name,
                                text,
                                b.labels
                                    .as_ref()
                                    .and_then(|l| l.get(ci))
                                    .and_then(|l| l.get(vi))
                                    .map(String::as_str),
                            )
                        {
                            break 'outer;
                        }
                    }
                }
            }
        }
        Ok(json!({"hits":hits,"truncated":truncated}))
    }
    fn annotation(
        &self,
        edition: &str,
        book: usize,
        chapter: usize,
        verse: usize,
    ) -> Result<Annotation> {
        let mut stmt = self.db.prepare_cached("SELECT payload FROM annotations WHERE edition=? AND book=? AND chapter=? AND verse=?").map_err(error)?;
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
        if [
            "books",
            "chapter",
            "search",
            "saved",
            "compare",
            "bookmark",
            "delete_annotation",
        ]
        .contains(&op)
        {
            for (id, data) in &mut self.editions {
                if id != edition
                    && (op != "compare"
                        || Some(id.as_str()) != request.get("other").and_then(Value::as_str))
                    && data.builtin.is_none()
                {
                    data.release();
                }
            }
        }
        match op {
            "compare" => self.compare(edition, field(&request,"other")?, index(&request,"book")?, index(&request,"chapter")?),
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
                    json!({"editions":self.editions.keys().collect::<Vec<_>>(), "defaultEdition":DEFAULT_EDITION, "settings":settings,
                        "editionMetadata":self.editions.iter().map(|(id,b)|json!({"id":id,"language":b.language,"books":b.book_count,"catholic":b.canon=="catholic","canon":b.canon})).collect::<Vec<_>>()}),
                )
            }
            "books" => Ok(json!(
                self.books(edition)?
                    .iter()
                    .enumerate()
                    .map(|(i, b)| json!({"index":i,"name":b.name,"abbrev":b.abbrev,"chapters":b.chapters}))
                    .collect::<Vec<_>>()
            )),
            "chapter" => {
                let book = index(&request, "book")?;
                let chapter = index(&request, "chapter")?;
                let verses = self.chapter(edition, book, chapter)?;
                let mut annotations = BTreeMap::new();
                let mut stmt = self.db.prepare_cached("SELECT verse,payload FROM annotations WHERE edition=? AND book=? AND chapter=?").map_err(error)?;
                let rows = stmt.query_map(params![edition,book as i64,chapter as i64],|r| Ok((r.get::<_,u32>(0)? as usize,r.get::<_,String>(1)?))).map_err(error)?;
                for row in rows { let (verse,payload) = row.map_err(error)?; annotations.insert(verse,serde_json::from_str::<Annotation>(&payload).map_err(error)?); }
                let mut result = Vec::with_capacity(verses.len());
                for i in 0..verses.len() {
                    let annotation = annotations.remove(&i).unwrap_or_default();
                    result.push(json!({"verse":i,"label":verses.label(i),"text":verses.get(i).unwrap(),"annotation":annotation}));
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
                let mut annotation: Annotation = serde_json::from_value(
                    request
                        .get("annotation")
                        .cloned()
                        .ok_or("Annotazione mancante")?,
                )
                .map_err(error)?;
                let old = self.annotation(edition, book, chapter, verse)?;
                if request["annotation"].get("noteColor").is_none() { annotation.note_color = old.note_color; }
                if request["annotation"].get("noteStyle").is_none() { annotation.note_style = old.note_style; }
                reading::validate_note(&annotation)?;
                if !selection::valid_color(&annotation.highlight) {
                    return Err("Colore non valido".into());
                }
                if annotation.note.len() > 32_768 {
                    return Err("Nota troppo lunga".into());
                }
                if annotation.ranges.len() > 512 {
                    return Err("Troppe annotazioni nel versetto".into());
                }
                for mark in &annotation.ranges {
                    selection::validate_range(
                        self.chapter(edition, book, chapter)?.get(verse).ok_or("Versetto non disponibile")?,
                        mark.start,
                        mark.length,
                    )?;
                    if !selection::valid_color(&mark.highlight) {
                        return Err("Colore non valido".into());
                    }
                }
                self.save_annotation(edition, book, chapter, verse, &annotation)?;
                Ok(json!(annotation))
            }
            "mark" => {
                let (book, chapter, verse) = (
                    index(&request, "book")?,
                    index(&request, "chapter")?,
                    index(&request, "verse")?,
                );
                let text = self
                    .chapter(edition, book, chapter)?
                    .get(verse)
                    .ok_or("Versetto non disponibile")?;
                let mut annotation = self.annotation(edition, book, chapter, verse)?;
                selection::apply_mark(
                    text,
                    &mut annotation,
                    index(&request, "start")?,
                    index(&request, "length")?,
                    field(&request, "action")?,
                )?;
                self.save_annotation(edition, book, chapter, verse, &annotation)?;
                Ok(json!(annotation))
            }
            "transfer" => self.transfer(&request),
            "export" => self.export_notes(&request),
            "reading" => self.reading(&request),
            "note" => self.write_note(&request,edition),
            "bookmark" => self.change_annotation(&request,edition,true),
            "delete_annotation" => self.change_annotation(&request,edition,false),
            "saved" => self.saved_notes(&request,edition),
            "collections" => self.collections(),
            "collection" => self.collection(&request),
            "organize" => self.organize(&request,edition),
            "search" => {
                let query = field(&request, "query")?.trim().to_lowercase();
                if query.chars().count() < 2 {
                    return Err("Inserisci almeno due caratteri".into());
                }
                if query.len() > 256 {
                    return Err("Ricerca troppo lunga".into());
                }
                self.search(edition,&query)
            }
            "settings" => {
                let key = field(&request, "key")?;
                if !["position", "fontSize", "theme", "language", "bibleLanguage", "comparisonEdition", "readingMargin", "focusMode"].contains(&key) {
                    return Err("Impostazione sconosciuta".into());
                }
                let value = request.get("value").ok_or("Valore mancante")?;
                if key == "comparisonEdition" && !value.as_str().is_some_and(|id| self.editions.contains_key(id)) {
                    return Err("Traduzione non disponibile".into());
                }
                if key == "language" && !value.as_str().is_some_and(|s| ["it", "en"].contains(&s)) {
                    return Err("Lingua non disponibile".into());
                }
                if key == "bibleLanguage"
                    && !value
                        .as_str()
                        .is_some_and(|s| ["it", "en", "all"].contains(&s))
                {
                    return Err("Lingua non disponibile".into());
                }
                if key == "fontSize" && !value.as_u64().is_some_and(|n| (14..=32).contains(&n)) {
                    return Err("Dimensione non valida".into());
                }
                if key == "theme"
                    && !value
                        .as_str()
                        .is_some_and(|s| ["Default", "Light", "Dark", "Sepia"].contains(&s))
                {
                    return Err("Tema non valido".into());
                }
                if key == "readingMargin" && !value.as_u64().is_some_and(|n| (16..=96).contains(&n)) {
                    return Err("Margini non validi".into());
                }
                if key == "focusMode" && !value.is_boolean() {
                    return Err("Modalità concentrazione non valida".into());
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
                self.db.prepare_cached("INSERT INTO settings VALUES (?,?) ON CONFLICT(key) DO UPDATE SET payload=excluded.payload WHERE payload<>excluded.payload").map_err(error)?
                    .execute(params![key, value.to_string()])
                    .map_err(error)?;
                if key == "position" { self.track_reading(value)?; }
                Ok(json!(true))
            }
            "import" => {
                let id = field(&request, "id")?.trim();
                if id.is_empty()
                    || id.len() > 80
                    || corpus::BUILTINS.iter().any(|b|b.id==id)
                {
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
                            || b.labels.as_ref().is_some_and(|labels| {
                                labels.len() != b.chapters.len() || labels.iter().zip(&b.chapters).any(|(ls, vs)|
                                    ls.len() != vs.len() || ls.iter().any(|l| l.trim().is_empty() || l.len() > 24)
                                    || ls.iter().collect::<std::collections::BTreeSet<_>>().len() != ls.len())
                            })
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
                    .execute("INSERT INTO editions (id,payload,book_count) VALUES (?,?,?)", params![id, payload, books.len() as i64])
                    .map_err(error)?;
                for cached in self.editions.values_mut() { cached.release(); }
                let data = Edition::imported(books.len());
                let _ = data.loaded.set(books);
                self.editions.insert(id.to_string(),data);
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
