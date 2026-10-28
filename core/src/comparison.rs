//! Reference alignment, shared with the Swift implementation through comparison.json.
use crate::{Engine, Result, corpus::Chapter};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

#[derive(Deserialize)]
struct Index {
    aliases: BTreeMap<String, String>,
    psalms: BTreeMap<String, Vec<Vec<String>>>,
}
fn data() -> &'static Index {
    static INDEX: OnceLock<Index> = OnceLock::new();
    INDEX.get_or_init(|| {
        serde_json::from_str(include_str!("../data/comparison.json"))
            .expect("validated comparison index")
    })
}
fn identity(name: &str) -> String {
    let key: String = name
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|c| c.is_alphanumeric())
        .collect();
    data()
        .aliases
        .get(&key)
        .cloned()
        .unwrap_or_else(|| format!("custom:{key}"))
}
fn file(edition: &str) -> &str {
    match edition {
        "CEI 2008" => "cei2008",
        "Martini 1781 (cattolica)" => "martini",
        "Diodati 1649" => "diodati",
        "KJV" => "kjv",
        "Raccolta estesa (83 sezioni)" => "extended",
        _ => "",
    }
}
fn key(edition: &str, code: &str, chapters: usize, chapter: usize, label: &str) -> String {
    if code == "Ps" {
        return label
            .parse::<usize>()
            .ok()
            .and_then(|v| {
                data()
                    .psalms
                    .get(file(edition))?
                    .get(chapter)?
                    .get(v.checked_sub(1)?)
            })
            .cloned()
            .unwrap_or_else(|| format!("unmapped:{edition}:{chapter}:{label}"));
    }
    let (mut c, mut v) = (chapter + 1, label.to_owned());
    if code == "Esth" && edition == "CEI 2008" {
        // CEI contains separately labelled Greek and Hebrew sections. Only an
        // explicit Hebrew reference can be paired with ordinary Esther.
        if let Some(hebrew) = label.strip_prefix('E') {
            v = hebrew.into();
        } else {
            return format!("unmapped:CEI:{chapter}:{label}");
        }
    }
    if code == "Joel" && chapters == 4 {
        if c == 3 {
            c = 2;
            v = label
                .parse::<usize>()
                .map_or(v.clone(), |n| (n + 27).to_string());
        } else if c == 4 {
            c = 3;
        }
    }
    if code == "Mal"
        && chapters == 3
        && c == 3
        && let Ok(n) = v.parse::<usize>()
        && n > 18
    {
        c = 4;
        v = (n - 18).to_string();
    }
    format!("{code}:{c}:{v}")
}
fn cell(
    edition: &str,
    book: usize,
    chapter: usize,
    verse: usize,
    name: &str,
    verses: &Chapter<'_>,
) -> Value {
    let label = verses.label(verse);
    json!({"edition":edition,"book":book,"chapter":chapter,"verse":verse,"label":label,
           "reference":format!("{} {}:{}",name,chapter+1,label),"text":verses.get(verse).unwrap()})
}
impl Engine {
    pub(crate) fn compare(
        &self,
        edition: &str,
        other: &str,
        book: usize,
        chapter: usize,
    ) -> Result<Value> {
        let books = self.books(edition)?;
        let source = books.get(book).ok_or("Libro non disponibile")?;
        let code = identity(source.name);
        let left = self.chapter(edition, book, chapter)?;
        let targets = self.books(other)?;
        let target = targets.iter().position(|b| identity(b.name) == code);
        let mut right = BTreeMap::<String, Vec<Value>>::new();
        let mut differing = false;
        let wanted: BTreeSet<_> = (0..left.len())
            .map(|v| key(edition, &code, source.chapters, chapter, &left.label(v)))
            .collect();
        if let Some(b) = target {
            let t = &targets[b];
            // Read only the chapters which contain a matching reference. The
            // catalog scan does not parse or copy a Bible's text.
            for c in 0..t.chapters {
                let candidate = self.chapter(other, b, c)?;
                let relevant = (0..candidate.len()).any(|v| {
                    wanted.contains(&key(other, &code, t.chapters, c, &candidate.label(v)))
                });
                if !relevant {
                    continue;
                }
                let conventional = !["Ps", "Joel", "Mal", "Esth"].contains(&code.as_str());
                if conventional
                    && c == chapter
                    && candidate.label(candidate.len() - 1) != left.label(left.len() - 1)
                {
                    differing = true;
                    continue;
                }
                for v in 0..candidate.len() {
                    let k = key(other, &code, t.chapters, c, &candidate.label(v));
                    if wanted.contains(&k) {
                        right
                            .entry(k)
                            .or_default()
                            .push(cell(other, b, c, v, t.name, &candidate));
                    }
                }
            }
        }
        let mut rows = Vec::<Value>::new();
        let mut groups = BTreeMap::<String, usize>::new();
        let mut missing = 0;
        for v in 0..left.len() {
            let k = key(edition, &code, source.chapters, chapter, &left.label(v));
            let value = cell(edition, book, chapter, v, source.name, &left);
            if let Some(&i) = groups.get(&k) {
                rows[i]["left"].as_array_mut().unwrap().push(value);
            } else {
                let paired = right.remove(&k).unwrap_or_default();
                if paired.is_empty() {
                    missing += 1;
                }
                groups.insert(k, rows.len());
                rows.push(json!({"left":[value],"right":paired}));
            }
        }
        let status = if target.is_none() {
            "missingBook"
        } else if differing || missing == rows.len() {
            "differentNumbering"
        } else if missing > 0 {
            "partial"
        } else {
            "aligned"
        };
        Ok(json!({"edition":edition,"other":other,"status":status,"rows":rows}))
    }
}
