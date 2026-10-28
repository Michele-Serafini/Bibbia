//! Bounded annotation browsing. Organization is separate from annotation payloads
//! so edits (and older compatible app versions) preserve folder/tag assignments.
use crate::{Annotation, Engine, Result, error, field, index};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(crate) const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS note_groups(kind TEXT NOT NULL CHECK(kind IN ('folder','tag')),name TEXT NOT NULL,PRIMARY KEY(kind,name));
CREATE TABLE IF NOT EXISTS note_organization(edition TEXT NOT NULL,book INTEGER NOT NULL,chapter INTEGER NOT NULL,verse INTEGER NOT NULL,folder TEXT NOT NULL DEFAULT '',tags TEXT NOT NULL DEFAULT '[]',PRIMARY KEY(edition,book,chapter,verse));
CREATE INDEX IF NOT EXISTS note_organization_folder ON note_organization(edition,folder,book,chapter,verse);";

#[derive(Default, Deserialize, Serialize)]
struct Organization {
    folder: String,
    tags: Vec<String>,
}
fn name(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 80 || value.chars().any(|c| c.is_control() || c == ',') {
        return Err(
            "Nome non valido: usa 1–80 byte, senza virgole o caratteri di controllo".into(),
        );
    }
    Ok(value.into())
}
impl Engine {
    pub(crate) fn collections(&self) -> Result<Value> {
        let mut folders = Vec::new();
        let mut tags = Vec::new();
        let mut statement = self
            .db
            .prepare_cached("SELECT kind,name FROM note_groups ORDER BY kind,name")
            .map_err(error)?;
        let rows = statement
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(error)?;
        for row in rows {
            let (kind, value) = row.map_err(error)?;
            if kind == "folder" {
                folders.push(value)
            } else {
                tags.push(value)
            }
        }
        Ok(json!({"folders":folders,"tags":tags}))
    }
    pub(crate) fn organize(&self, request: &Value, edition: &str) -> Result<Value> {
        let (b, c, v) = (
            index(request, "book")?,
            index(request, "chapter")?,
            index(request, "verse")?,
        );
        let exists:bool=self.db.query_row("SELECT EXISTS(SELECT 1 FROM annotations WHERE edition=? AND book=? AND chapter=? AND verse=?)",params![edition,b as i64,c as i64,v as i64],|r|r.get(0)).map_err(error)?;
        if !exists {
            return Err("Salva prima una nota, un segnalibro o un’evidenziazione".into());
        }
        let folder = field(request, "folder")?.trim();
        let folder = if folder.is_empty() {
            String::new()
        } else {
            name(folder)?
        };
        let values = request
            .get("tags")
            .and_then(Value::as_array)
            .ok_or("Etichette non valide")?;
        if values.len() > 16 {
            return Err("Al massimo 16 etichette per appunto".into());
        }
        let tags = values
            .iter()
            .map(|v| name(v.as_str().ok_or("Etichetta non valida")?))
            .collect::<Result<BTreeSet<_>>>()?
            .into_iter()
            .collect::<Vec<_>>();
        let tx = self.db.unchecked_transaction().map_err(error)?;
        if !folder.is_empty() {
            tx.execute(
                "INSERT OR IGNORE INTO note_groups VALUES('folder',?)",
                [&folder],
            )
            .map_err(error)?;
        }
        for tag in &tags {
            tx.execute("INSERT OR IGNORE INTO note_groups VALUES('tag',?)", [tag])
                .map_err(error)?;
        }
        let result = Organization { folder, tags };
        tx.execute("INSERT INTO note_organization VALUES(?,?,?,?,?,?) ON CONFLICT(edition,book,chapter,verse) DO UPDATE SET folder=excluded.folder,tags=excluded.tags",params![edition,b as i64,c as i64,v as i64,result.folder,serde_json::to_string(&result.tags).map_err(error)?]).map_err(error)?;
        tx.commit().map_err(error)?;
        Ok(json!(result))
    }
    pub(crate) fn collection(&self, request: &Value) -> Result<Value> {
        let kind = field(request, "kind")?;
        if !["folder", "tag"].contains(&kind) {
            return Err("Tipo raccolta non valido".into());
        }
        let old = name(field(request, "name")?)?;
        let action = field(request, "action")?;
        let tx = self.db.unchecked_transaction().map_err(error)?;
        match action {
            "create" => {
                tx.execute(
                    "INSERT OR IGNORE INTO note_groups VALUES(?,?)",
                    params![kind, old],
                )
                .map_err(error)?;
            }
            "rename" | "delete" => {
                let exists: bool = tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM note_groups WHERE kind=? AND name=?)",
                        params![kind, old],
                        |r| r.get(0),
                    )
                    .map_err(error)?;
                if !exists {
                    return Err("Cartella o etichetta non disponibile".into());
                }
                let new = if action == "rename" {
                    name(field(request, "newName")?)?
                } else {
                    String::new()
                };
                if new == old {
                    tx.commit().map_err(error)?;
                    return self.collections();
                }
                if action == "rename" {
                    tx.execute("INSERT INTO note_groups VALUES(?,?)", params![kind, new])
                        .map_err(error)?;
                }
                if kind == "folder" {
                    tx.execute(
                        "UPDATE note_organization SET folder=? WHERE folder=?",
                        params![new, old],
                    )
                    .map_err(error)?;
                } else if action == "rename" {
                    tx.execute("UPDATE note_organization SET tags=(SELECT json_group_array(CASE WHEN value=?1 THEN ?2 ELSE value END) FROM json_each(note_organization.tags)) WHERE EXISTS(SELECT 1 FROM json_each(note_organization.tags) WHERE value=?1)",params![old,new]).map_err(error)?;
                } else {
                    tx.execute("UPDATE note_organization SET tags=(SELECT json_group_array(value) FROM json_each(note_organization.tags) WHERE value<>?1) WHERE EXISTS(SELECT 1 FROM json_each(note_organization.tags) WHERE value=?1)",[&old]).map_err(error)?;
                }
                tx.execute(
                    "DELETE FROM note_groups WHERE kind=? AND name=?",
                    params![kind, old],
                )
                .map_err(error)?;
            }
            _ => return Err("Azione raccolta non valida".into()),
        }
        tx.commit().map_err(error)?;
        self.collections()
    }
    pub(crate) fn saved_notes(&self, request: &Value, edition: &str) -> Result<Value> {
        let books = self.books(edition)?;
        let limit = request.get("limit").map_or(Ok(200), |v| {
            v.as_i64()
                .filter(|v| (1..=200).contains(v))
                .ok_or("Limite non valido")
        })? as usize;
        let offset = request
            .get("offset")
            .map_or(Ok(0), |v| v.as_u64().ok_or("Indice non valido: offset"))?;
        let query = request
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_lowercase();
        if query.len() > 1024 {
            return Err("Ricerca troppo lunga".into());
        }
        let folder = request.get("folder").and_then(Value::as_str);
        let tag = request.get("tag").and_then(Value::as_str);
        let kind = request.get("kind").and_then(Value::as_str).unwrap_or("all");
        if !["all", "note", "marked", "bookmark"].contains(&kind) {
            return Err("Filtro appunti non valido".into());
        }
        let mut statement=self.db.prepare_cached("SELECT a.book,a.chapter,a.verse,a.payload,COALESCE(o.folder,''),COALESCE(o.tags,'[]') FROM annotations a LEFT JOIN note_organization o USING(edition,book,chapter,verse) WHERE a.edition=?1 AND (?2 IS NULL OR COALESCE(o.folder,'')=?2) AND (?3 IS NULL OR EXISTS(SELECT 1 FROM json_each(COALESCE(o.tags,'[]')) WHERE value=?3)) ORDER BY a.book,a.chapter,a.verse").map_err(error)?;
        let mut rows = statement
            .query(params![edition, folder, tag])
            .map_err(error)?;
        let mut result = Vec::new();
        let mut skipped = 0;
        while let Some(row) = rows.next().map_err(error)? {
            let (b, c, v) = (
                row.get::<_, i64>(0).map_err(error)? as usize,
                row.get::<_, i64>(1).map_err(error)? as usize,
                row.get::<_, i64>(2).map_err(error)? as usize,
            );
            let annotation: Annotation =
                serde_json::from_str(&row.get::<_, String>(3).map_err(error)?).map_err(error)?;
            let marked = annotation.highlight.is_some()
                || annotation.underline
                || !annotation.ranges.is_empty();
            if (kind == "note" && annotation.note.is_empty())
                || (kind == "marked" && !marked)
                || (kind == "bookmark" && !annotation.bookmark)
            {
                continue;
            }
            let name = books.get(b).ok_or("Libro salvato non valido")?.name;
            let chapter = self.chapter(edition, b, c)?;
            let text = chapter.get(v).ok_or("Versetto salvato non valido")?;
            let folder: String = row.get(4).map_err(error)?;
            let tags: Vec<String> =
                serde_json::from_str(&row.get::<_, String>(5).map_err(error)?).map_err(error)?;
            let reference = format!("{name} {}:{}", c + 1, chapter.label(v));
            if !query.is_empty()
                && ![
                    text,
                    annotation.note.as_str(),
                    reference.as_str(),
                    folder.as_str(),
                ]
                .into_iter()
                .any(|s| s.to_lowercase().contains(&query))
                && !tags.iter().any(|s| s.to_lowercase().contains(&query))
            {
                continue;
            }
            if skipped < offset {
                skipped += 1;
                continue;
            }
            result.push(json!({"book":b,"chapter":c,"verse":v,"name":name,"label":chapter.label(v),"text":text,"annotation":annotation,"organization":{"folder":folder,"tags":tags}}));
            if result.len() == limit {
                break;
            }
        }
        Ok(json!(result))
    }
}
