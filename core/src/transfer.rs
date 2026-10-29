//! Consistent SQLite snapshots and streaming human-readable annotation exports.
use crate::{Annotation, Engine, Result, error, field};
use rusqlite::{Connection, OpenFlags, backup::Backup};
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
const APP_ID: i64 = 0x42494242;
const PENDING: &str = "pending-restore.sqlite3";
const TABLES: [(&str, &[&str]); 7] = [
    (
        "annotations",
        &["edition", "book", "chapter", "verse", "payload"],
    ),
    ("settings", &["key", "payload"]),
    ("editions", &["id", "payload", "book_count"]),
    ("note_groups", &["kind", "name"]),
    (
        "note_organization",
        &["edition", "book", "chapter", "verse", "folder", "tags"],
    ),
    ("reading_plans", &["id", "payload"]),
    ("reading_progress", &["plan_id", "chapter_index"]),
];
fn nonce() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
fn temporary(path: &Path) -> PathBuf {
    path.with_file_name(format!(".bibbia-{}-{}.tmp", std::process::id(), nonce()))
}
fn reserved(path: &Path, directory: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or("Percorso non valido")?
        .canonicalize()
        .map_err(error)?;
    let name = path
        .file_name()
        .ok_or("Percorso non valido")?
        .to_string_lossy();
    let current = directory.canonicalize().map_err(error)?;
    if (parent == current
        && [
            "bible.sqlite3",
            "bible.sqlite3-wal",
            "bible.sqlite3-shm",
            PENDING,
            "reader.json",
        ]
        .contains(&name.as_ref()))
        || path.is_symlink()
    {
        return Err("Scegli un file diverso dai dati dell’app".into());
    }
    Ok(())
}
fn create_file(path: &Path) -> Result<File> {
    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    opts.open(path).map_err(error)
}
fn publish(temp: &Path, path: &Path) -> Result<()> {
    #[cfg(not(windows))]
    {
        fs::rename(temp, path).map_err(error)
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
        }
        let from: Vec<_> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0x1 | 0x8) } == 0 {
            Err(error(std::io::Error::last_os_error()))
        } else {
            Ok(())
        }
    }
}
fn snapshot(source: &Connection, path: &Path) -> Result<()> {
    let temp = temporary(path);
    drop(create_file(&temp)?);
    let result = (|| {
        let mut target = Connection::open(&temp).map_err(error)?;
        Backup::new(source, &mut target)
            .map_err(error)?
            .run_to_completion(256, Duration::from_millis(10), None)
            .map_err(error)?;
        target
            .execute_batch(&format!(
                "PRAGMA journal_mode=DELETE;PRAGMA application_id={APP_ID};PRAGMA user_version=1;"
            ))
            .map_err(error)?;
        drop(target);
        OpenOptions::new()
            .write(true)
            .open(&temp)
            .map_err(error)?
            .sync_all()
            .map_err(error)?;
        publish(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}
fn inspect(path: &Path) -> Result<(Connection, Value)> {
    let len = fs::metadata(path).map_err(error)?.len();
    if !(100..=1_073_741_824).contains(&len) {
        return Err("Dimensione backup non valida (massimo 1 GiB)".into());
    }
    let db = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(error)?;
    db.execute_batch("PRAGMA trusted_schema=OFF;PRAGMA query_only=ON;BEGIN;")
        .map_err(error)?;
    let app: i64 = db
        .query_row("PRAGMA application_id", [], |r| r.get(0))
        .map_err(error)?;
    let version: i64 = db
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(error)?;
    if app != APP_ID || version != 1 {
        return Err("Non è un backup Bibbia supportato".into());
    }
    let ok: String = db
        .query_row("PRAGMA quick_check", [], |r| r.get(0))
        .map_err(error)?;
    if ok != "ok" {
        return Err("Backup danneggiato".into());
    }
    let mut schema = db
        .prepare("SELECT type,name,sql FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'")
        .map_err(error)?;
    let mut tables = 0;
    let rows = schema
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(error)?;
    for row in rows {
        let (kind, name, sql) = row.map_err(error)?;
        if kind == "index" {
            continue;
        }
        if kind != "table"
            || !TABLES.iter().any(|t| t.0 == name)
            || sql.to_lowercase().contains("virtual")
        {
            return Err("Schema backup non valido".into());
        }
        tables += 1;
    }
    if tables != 7 {
        return Err("Backup incompleto".into());
    }
    for (table, columns) in TABLES {
        let mut stmt = db
            .prepare(&format!("PRAGMA table_info({table})"))
            .map_err(error)?;
        let names = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(error)?;
        if names != columns {
            return Err("Schema backup non compatibile".into());
        }
    }
    for table in ["annotations", "settings", "editions", "reading_plans"] {
        let invalid: bool = db
            .query_row(
                &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE NOT json_valid(payload))"),
                [],
                |r| r.get(0),
            )
            .map_err(error)?;
        if invalid {
            return Err("Dati backup non validi".into());
        }
    }
    let invalid: bool = db
        .prepare(include_str!("../data/backup-validation.sql"))
        .map_err(error)?
        .exists([])
        .map_err(error)?;
    if invalid {
        return Err("Dati backup non validi".into());
    }
    let mut counts = serde_json::Map::new();
    for table in ["annotations", "editions", "reading_plans", "note_groups"] {
        let n: i64 = db
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .map_err(error)?;
        counts.insert(table.into(), json!(n));
    }
    drop(schema);
    Ok((db, json!(counts)))
}
pub(super) fn apply_pending(target: &mut Connection, directory: &Path) -> Result<()> {
    let pending = directory.join(PENDING);
    if !pending.exists() {
        return Ok(());
    }
    let (source, _) = inspect(&pending)?;
    let safety = directory.join(format!("Bibbia-before-restore-{}.bibbia-backup", nonce()));
    snapshot(target, &safety)?;
    Backup::new(&source, target)
        .map_err(error)?
        .run_to_completion(256, Duration::from_millis(10), None)
        .map_err(error)?;
    // Reader page geometry is specific to the destination platform and viewport.
    drop(source);
    let _ = fs::remove_file(directory.join("reader.json"));
    fs::remove_file(pending).map_err(error)?;
    Ok(())
}
impl Engine {
    pub(crate) fn transfer(&self, r: &Value) -> Result<Value> {
        let path = Path::new(field(r, "path")?);
        reserved(path, &self.directory)?;
        match field(r, "action")? {
            "backup" => {
                snapshot(&self.db, path)?;
                Ok(json!(true))
            }
            "inspect" => Ok(inspect(path)?.1),
            "restore" => {
                let (source, counts) = inspect(path)?;
                snapshot(&source, &self.directory.join(PENDING))?;
                Ok(counts)
            }
            _ => Err("Operazione backup non valida".into()),
        }
    }
    pub(crate) fn export_notes(&mut self, r: &Value) -> Result<Value> {
        let path = Path::new(field(r, "path")?);
        reserved(path, &self.directory)?;
        let markdown = match field(r, "format")? {
            "markdown" => true,
            "text" => false,
            _ => return Err("Formato non valido".into()),
        };
        let temp = temporary(path);
        let result = (|| {
            let mut out = BufWriter::new(create_file(&temp)?);
            writeln!(
                out,
                "{}Bibbia · Appunti\n",
                if markdown { "# " } else { "" }
            )
            .map_err(error)?;
            let mut stmt=self.db.prepare("SELECT a.edition,a.book,a.chapter,a.verse,a.payload,COALESCE(o.folder,''),COALESCE(o.tags,'[]') FROM annotations a LEFT JOIN note_organization o USING(edition,book,chapter,verse) ORDER BY a.edition,a.book,a.chapter,a.verse").map_err(error)?;
            let mut rows = stmt.query([]).map_err(error)?;
            let mut count = 0;
            let mut cei = false;
            let mut current_edition = String::new();
            while let Some(row) = rows.next().map_err(error)? {
                let edition: String = row.get(0).map_err(error)?;
                let b = row.get::<_, u32>(1).map_err(error)? as usize;
                let c = row.get::<_, u32>(2).map_err(error)? as usize;
                let v = row.get::<_, u32>(3).map_err(error)? as usize;
                let a: Annotation = serde_json::from_str(&row.get::<_, String>(4).map_err(error)?)
                    .map_err(error)?;
                let folder: String = row.get(5).map_err(error)?;
                let tags: Vec<String> =
                    serde_json::from_str(&row.get::<_, String>(6).map_err(error)?)
                        .map_err(error)?;
                if current_edition != edition {
                    for (id, data) in &mut self.editions {
                        if id != &edition {
                            data.release();
                        }
                    }
                    current_edition = edition.clone();
                }
                let books = self.books(&edition)?;
                let name = books.get(b).ok_or("Riferimento non disponibile")?.name;
                let chapter = self.chapter(&edition, b, c)?;
                let text = chapter.get(v).ok_or("Versetto non disponibile")?;
                let render = |s: &str| if markdown { escape(s) } else { s.into() };
                writeln!(
                    out,
                    "{}{} {}:{} · {}\n",
                    if markdown { "## " } else { "" },
                    render(name),
                    c + 1,
                    render(&chapter.label(v)),
                    render(&edition)
                )
                .map_err(error)?;
                writeln!(
                    out,
                    "{}{}\n",
                    if markdown { "> " } else { "" },
                    render(text)
                )
                .map_err(error)?;
                if !a.note.is_empty() {
                    writeln!(out, "{}\n", render(&a.note)).map_err(error)?;
                }
                let mut labels = Vec::new();
                if a.bookmark {
                    labels.push("Segnalibro / Bookmark".into())
                }
                if a.underline {
                    labels.push("Sottolineato / Underlined".into())
                }
                if let Some(color) = a.highlight {
                    labels.push(format!("Evidenziazione / Highlight: {color}"))
                }
                if let Some(color) = a.note_color {
                    labels.push(format!("Colore nota / Note color: {color}"))
                }
                if a.note_style.as_deref() == Some("sticky") {
                    labels.push("Post-it".into())
                }
                if !folder.is_empty() {
                    labels.push(format!("Cartella / Folder: {folder}"))
                }
                if !tags.is_empty() {
                    labels.push(format!("Etichette / Tags: {}", tags.join(", ")))
                }
                if !labels.is_empty() {
                    writeln!(out, "{}\n", render(&labels.join(" · "))).map_err(error)?;
                }
                for mark in a.ranges {
                    crate::selection::validate_range(text, mark.start, mark.length)?;
                    let selected = String::from_utf16(
                        &text
                            .encode_utf16()
                            .skip(mark.start)
                            .take(mark.length)
                            .collect::<Vec<_>>(),
                    )
                    .map_err(error)?;
                    writeln!(
                        out,
                        "{}«{}» · {}{}\n",
                        if markdown { "- " } else { "" },
                        render(&selected),
                        if mark.underline {
                            "Sottolineato / Underlined "
                        } else {
                            ""
                        },
                        mark.highlight.as_deref().unwrap_or("")
                    )
                    .map_err(error)?;
                }
                writeln!(out, "---\n").map_err(error)?;
                count += 1;
                cei |= edition == "CEI 2008";
            }
            if count == 0 {
                writeln!(out, "Nessun appunto salvato / No saved annotations.").map_err(error)?;
            }
            if cei {
                writeln!(out,"CEI 2008 · © Fondazione di Religione Santi Francesco d’Assisi e Caterina da Siena. Uso formativo e didattico non commerciale. Fonte: BibbiaEDU.").map_err(error)?;
            }
            out.flush().map_err(error)?;
            out.get_ref().sync_all().map_err(error)?;
            drop(out);
            publish(&temp, path)?;
            Ok(json!({"annotations":count}))
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result
    }
}
fn escape(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if "\\`*_{}[]<>()#+-.!|~".contains(c) {
                vec!['\\', c]
            } else {
                vec![c]
            }
        })
        .collect()
}
#[cfg(test)]
pub(super) fn test_inspect(path: &Path) -> Result<(Connection, Value)> {
    inspect(path)
}
