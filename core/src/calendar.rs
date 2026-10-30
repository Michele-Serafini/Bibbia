//! RFC 5545 all-day snapshots for the user's default calendar application.
use crate::{Engine, Result, error};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
pub(crate) struct CalendarPlan<'a> {
    pub id: i64,
    pub title: &'a str,
    pub edition: &'a str,
    pub start: &'a str,
    pub daily: usize,
    pub book: Option<usize>,
    pub chapters: &'a [(usize, usize, &'a str)],
    pub completed: &'a BTreeSet<usize>,
    pub english: bool,
}
fn civil(z: i64) -> (i64, i64, i64) {
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    y += i64::from(m <= 2);
    (y, m, d)
}
fn date(z: i64) -> String {
    let (y, m, d) = civil(z);
    format!("{y:04}{m:02}{d:02}")
}
fn text(value: &str) -> String {
    let mut out = String::new();
    for c in value.replace("\r\n", "\n").replace('\r', "\n").chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\n' => out.push_str("\\n"),
            c if !c.is_control() => out.push(c),
            _ => {}
        }
    }
    out
}
fn line(out: &mut dyn Write, value: &str) -> Result<()> {
    let mut rest = value;
    let mut first = true;
    while !rest.is_empty() {
        let budget = if first { 75 } else { 74 };
        let mut n = rest.len().min(budget);
        while !rest.is_char_boundary(n) {
            n -= 1;
        }
        if !first {
            out.write_all(b" ").map_err(error)?;
        }
        out.write_all(&rest.as_bytes()[..n]).map_err(error)?;
        out.write_all(b"\r\n").map_err(error)?;
        rest = &rest[n..];
        first = false;
    }
    Ok(())
}
pub(crate) fn export(engine: &Engine, path: &Path, p: CalendarPlan<'_>) -> Result<Value> {
    let start = i64::from(crate::reading::day(p.start)?);
    let identity = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        p.id,
        p.edition,
        p.start,
        p.daily,
        p.book.map_or(-1, |b| b as i64),
        p.title
    );
    let hash = identity.bytes().fold(0xcbf29ce484222325u64, |h, c| {
        (h ^ u64::from(c)).wrapping_mul(0x100000001b3)
    });
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(error)?
        .as_secs();
    let sec = seconds % 86400;
    let stamp = format!(
        "{}T{:02}{:02}{:02}Z",
        date(i64::from(crate::reading::day("1970-01-01")?) + (seconds / 86400) as i64),
        sec / 3600,
        sec / 60 % 60,
        sec % 60
    );
    engine.write_document(path,|out|{
        for header in ["BEGIN:VCALENDAR","VERSION:2.0","PRODID:-//Bibbia//Reading Plans//IT","CALSCALE:GREGORIAN","METHOD:PUBLISH"]{line(out,header)?;}
        line(out,&format!("X-WR-CALNAME:{}",text(p.title)))?;
        for (day,chapters) in p.chapters.chunks(p.daily).enumerate(){
            let references=chapters.iter().map(|(_,c,name)|format!("{name} {}",c+1)).collect::<Vec<_>>().join(" · ");
            let count=(day*p.daily..day*p.daily+chapters.len()).filter(|i|p.completed.contains(i)).count();
            let description=if p.english{format!("{}\nReading: {references}\nCompleted: {count}/{}\nOpen Bibbia → Reading plans → Resume.",p.edition,chapters.len())}else{format!("{}\nLetture: {references}\nCompletati: {count}/{}\nApri Bibbia → Piani di lettura → Riprendi.",p.edition,chapters.len())};
            for property in ["BEGIN:VEVENT".into(),format!("UID:bibbia-{hash:016x}-{day}@bibbia.local"),format!("DTSTAMP:{stamp}"),format!("DTSTART;VALUE=DATE:{}",date(start+day as i64)),format!("DTEND;VALUE=DATE:{}",date(start+day as i64+1)),format!("SUMMARY:{}",text(&format!("{} · {references}",p.title))),format!("DESCRIPTION:{}",text(&description)),"TRANSP:TRANSPARENT".into(),"STATUS:CONFIRMED".into(),"END:VEVENT".into()]{line(out,&property)?;}
        }
        line(out,"END:VCALENDAR")
    })?;
    Ok(json!({"events":p.chapters.len().div_ceil(p.daily)}))
}
