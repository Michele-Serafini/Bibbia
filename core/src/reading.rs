//! Bounded offline reading schedules. Calendar dates are civil days (never seconds/DST).
use crate::{Annotation, Engine, Result, error, field, index};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;
pub(crate) const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS reading_plans(id INTEGER PRIMARY KEY,payload TEXT NOT NULL); CREATE TABLE IF NOT EXISTS reading_progress(plan_id INTEGER NOT NULL REFERENCES reading_plans(id) ON DELETE CASCADE,chapter_index INTEGER NOT NULL,PRIMARY KEY(plan_id,chapter_index));";
#[derive(Serialize, Deserialize)]
struct Plan {
    title: String,
    edition: String,
    start: String,
    daily: usize,
    book: Option<usize>,
    resume: Value,
}
// Gregorian day number, valid for the bounded 1900–2199 UI range.
fn ordinal(y: i32, m: i32, d: i32) -> i32 {
    let y = y - i32::from(m <= 2);
    let era = y / 400;
    let yy = y - era * 400;
    let mm = m + if m > 2 { -3 } else { 9 };
    era * 146097 + yy * 365 + yy / 4 - yy / 100 + (153 * mm + 2) / 5 + d - 1
}
pub(crate) fn day(s: &str) -> Result<i32> {
    let b = s.as_bytes();
    if b.len() != 10
        || b[4] != b'-'
        || b[7] != b'-'
        || b.iter()
            .enumerate()
            .any(|(i, c)| i != 4 && i != 7 && !c.is_ascii_digit())
    {
        return Err("Data non valida (AAAA-MM-GG)".into());
    }
    let y = s[..4].parse::<i32>().map_err(error)?;
    let m = s[5..7].parse::<i32>().map_err(error)?;
    let d = s[8..].parse::<i32>().map_err(error)?;
    let days = if m == 2 {
        if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            29
        } else {
            28
        }
    } else if [4, 6, 9, 11].contains(&m) {
        30
    } else {
        31
    };
    if !(1900..=2199).contains(&y) || !(1..=12).contains(&m) || !(1..=days).contains(&d) {
        return Err("Data non valida".into());
    }
    Ok(ordinal(y, m, d))
}
pub(crate) fn validate_note(a: &Annotation) -> Result<()> {
    if !crate::selection::valid_color(&a.note_color)
        || a.note_style
            .as_deref()
            .is_some_and(|s| !["plain", "sticky"].contains(&s))
    {
        return Err("Stile nota non valido".into());
    }
    Ok(())
}
impl Engine {
    pub(crate) fn write_note(&self, r: &Value, edition: &str) -> Result<Value> {
        let (b, c, v) = (index(r, "book")?, index(r, "chapter")?, index(r, "verse")?);
        self.chapter(edition, b, c)?
            .get(v)
            .ok_or("Versetto non disponibile")?;
        let mut a = self.annotation(edition, b, c, v)?;
        a.note = field(r, "text")?.into();
        if a.note.len() > 32768 {
            return Err("Nota troppo lunga".into());
        }
        a.note_color = r
            .get("color")
            .filter(|v| !v.is_null())
            .map(|v| v.as_str().map(str::to_owned).ok_or("Colore non valido"))
            .transpose()?;
        a.note_style = Some(field(r, "style")?.into());
        validate_note(&a)?;
        if a.note.is_empty() {
            a.note_color = None;
            a.note_style = None;
        }
        self.save_annotation(edition, b, c, v, &a)?;
        Ok(json!(a))
    }
    pub(crate) fn track_reading(&self, p: &Value) -> Result<()> {
        self.db.prepare_cached("UPDATE reading_plans SET payload=json_set(payload,'$.resume',json(?1)) WHERE id=COALESCE((SELECT CAST(payload AS INTEGER) FROM settings WHERE key='activeReadingPlan'),-1) AND json_extract(payload,'$.edition')=?2 AND (json_extract(payload,'$.book') IS NULL OR json_extract(payload,'$.book')=?3) AND (json_extract(payload,'$.resume.book')<>json_extract(?1,'$.book') OR json_extract(payload,'$.resume.chapter')<>json_extract(?1,'$.chapter') OR json_extract(payload,'$.resume.verse')<>json_extract(?1,'$.verse'))").map_err(error)?.execute(params![p.to_string(),field(p,"edition")?,index(p,"book")? as i64]).map_err(error)?;
        Ok(())
    }
    pub(crate) fn reading(&mut self, r: &Value) -> Result<Value> {
        let action = field(r, "action")?;
        if action == "list" {
            let mut q = self
                .db
                .prepare_cached("SELECT id,payload FROM reading_plans ORDER BY id DESC LIMIT 24")
                .map_err(error)?;
            let rows = q
                .query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(error)?;
            let mut plans = Vec::new();
            for row in rows {
                let (id, payload) = row.map_err(error)?;
                let p: Plan = serde_json::from_str(&payload).map_err(error)?;
                plans.push(json!({"id":id,"title":p.title,"edition":p.edition}));
            }
            return Ok(json!(plans));
        }
        if action == "create" {
            let count: i64 = self
                .db
                .query_row("SELECT COUNT(*) FROM reading_plans", [], |r| r.get(0))
                .map_err(error)?;
            if count >= 24 {
                return Err("Al massimo 24 piani".into());
            }
            let edition = field(r, "edition")?;
            let title = field(r, "title")?.trim();
            if title.is_empty() || title.len() > 160 {
                return Err("Titolo non valido".into());
            }
            let start = field(r, "start")?;
            day(start)?;
            let daily = index(r, "daily")?;
            if !(1..=12).contains(&daily) {
                return Err("Scegli 1–12 capitoli al giorno".into());
            }
            let book = r
                .get("book")
                .filter(|v| !v.is_null())
                .map(|_| index(r, "book"))
                .transpose()?;
            self.keep_imported(edition);
            let books = self.books(edition)?;
            if book.is_some_and(|b| b >= books.len()) {
                return Err("Libro non disponibile".into());
            }
            let p = Plan {
                title: title.into(),
                edition: edition.into(),
                start: start.into(),
                daily,
                book,
                resume: json!({"edition":edition,"book":book.unwrap_or(0),"chapter":0,"verse":0}),
            };
            self.db
                .execute(
                    "INSERT INTO reading_plans(payload) VALUES(?)",
                    [serde_json::to_string(&p).map_err(error)?],
                )
                .map_err(error)?;
            return Ok(json!(self.db.last_insert_rowid()));
        }
        let id = index(r, "id")? as i64;
        let payload: String = self
            .db
            .query_row("SELECT payload FROM reading_plans WHERE id=?", [id], |r| {
                r.get(0)
            })
            .map_err(error)?;
        let p: Plan = serde_json::from_str(&payload).map_err(error)?;
        if action == "delete" {
            let tx = self.db.unchecked_transaction().map_err(error)?;
            tx.execute("DELETE FROM reading_plans WHERE id=?", [id])
                .map_err(error)?;
            tx.execute(
                "DELETE FROM settings WHERE key='activeReadingPlan' AND CAST(payload AS INTEGER)=?",
                [id],
            )
            .map_err(error)?;
            tx.commit().map_err(error)?;
            return Ok(json!(true));
        }
        if action == "resume" || action == "activate" {
            self.db.execute("INSERT INTO settings VALUES('activeReadingPlan',?1) ON CONFLICT(key) DO UPDATE SET payload=excluded.payload WHERE payload<>excluded.payload",[id.to_string()]).map_err(error)?;
            return Ok(if action == "resume" {
                p.resume
            } else {
                json!(true)
            });
        }
        self.keep_imported(&p.edition);
        let books = self.books(&p.edition)?;
        let chapters: Vec<_> = books
            .iter()
            .enumerate()
            .filter(|(b, _)| p.book.is_none_or(|i| i == *b))
            .flat_map(|(b, book)| (0..book.chapters).map(move |c| (b, c, book.name)))
            .collect();
        if action == "calendar" {
            let completed = self
                .db
                .prepare_cached("SELECT chapter_index FROM reading_progress WHERE plan_id=?")
                .map_err(error)?
                .query_map([id], |row| row.get::<_, u32>(0).map(|v| v as usize))
                .map_err(error)?
                .collect::<std::result::Result<BTreeSet<_>, _>>()
                .map_err(error)?;
            let plan = crate::calendar::CalendarPlan {
                id,
                title: &p.title,
                edition: &p.edition,
                start: &p.start,
                daily: p.daily,
                book: p.book,
                chapters: &chapters,
                completed: &completed,
                english: r["language"].as_str() == Some("en"),
            };
            return crate::calendar::export(self, std::path::Path::new(field(r, "path")?), plan);
        }
        let mut q = self
            .db
            .prepare_cached(
                "SELECT chapter_index FROM reading_progress WHERE plan_id=? ORDER BY chapter_index",
            )
            .map_err(error)?;
        let mut done = q
            .query_map([id], |row| row.get::<_, u32>(0).map(|v| v as usize))
            .map_err(error)?
            .collect::<std::result::Result<BTreeSet<_>, _>>()
            .map_err(error)?;
        if action == "complete" {
            let i = index(r, "index")?;
            if i >= chapters.len() {
                return Err("Capitolo non disponibile".into());
            }
            let complete = r
                .get("complete")
                .and_then(Value::as_bool)
                .ok_or("Progresso non valido")?;
            let tx = self.db.unchecked_transaction().map_err(error)?;
            if complete {
                tx.execute(
                    "INSERT OR IGNORE INTO reading_progress VALUES(?,?)",
                    params![id, i as i64],
                )
                .map_err(error)?;
                done.insert(i);
            } else {
                tx.execute(
                    "DELETE FROM reading_progress WHERE plan_id=? AND chapter_index=?",
                    params![id, i as i64],
                )
                .map_err(error)?;
                done.remove(&i);
            }
            if complete && let Some(next) = (0..chapters.len()).find(|n| !done.contains(n)) {
                let (b, c, _) = chapters[next];
                let resume = json!({"edition":p.edition,"book":b,"chapter":c,"verse":0});
                tx.execute("UPDATE reading_plans SET payload=json_set(payload,'$.resume',json(?)) WHERE id=?",params![resume.to_string(),id]).map_err(error)?;
            }
            tx.commit().map_err(error)?;
            return Ok(json!(true));
        }
        if action != "detail" {
            return Err("Azione piano non valida".into());
        }
        let from = day(field(r, "from")?)?;
        let selected = day(field(r, "selected")?)?;
        let start = day(&p.start)?;
        let total_days = chapters.len().div_ceil(p.daily);
        let days: Vec<_> = (0..42)
            .map(|offset| {
                let n = from + offset - start;
                let first = n.max(0) as usize * p.daily;
                let end = (first + p.daily).min(chapters.len());
                let total = if n < 0 || first >= chapters.len() {
                    0
                } else {
                    end - first
                };
                let count = if total == 0 {
                    0
                } else {
                    done.range(first..end).count()
                };
                json!({"offset":offset,"total":total,"done":count})
            })
            .collect();
        let n = selected - start;
        let first = n.max(0) as usize * p.daily;
        let end = (first + p.daily).min(chapters.len());
        let items: Vec<_> = if n < 0 || first >= chapters.len() {
            Vec::new()
        } else {
            (first..end)
                .map(|i| {
                    let (b, c, name) = chapters[i];
                    json!({"index":i,"book":b,"chapter":c,"name":name,"complete":done.contains(&i)})
                })
                .collect()
        };
        Ok(
            json!({"id":id,"title":p.title,"edition":p.edition,"start":p.start,"daily":p.daily,"total":chapters.len(),"completed":done.len(),"duration":total_days,"days":days,"chapters":items,"resume":p.resume}),
        )
    }
}

impl Engine {
    fn keep_imported(&mut self, edition: &str) {
        for (id, data) in &mut self.editions {
            if id != edition && data.builtin.is_none() {
                data.release();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn civil_dates_validate_leap_and_invalid_input() {
        assert_eq!(day("2024-03-01").unwrap() - day("2024-02-28").unwrap(), 2);
        for s in [
            "2023-02-29",
            "1900-02-29",
            "2024-13-01",
            "2024-01-00",
            "🌅-02-03",
            "2024-2-03",
        ] {
            assert!(day(s).is_err(), "{s}");
        }
        assert!(day("2000-02-29").is_ok());
    }
}
