use bible_core::Annotation;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Position {
    pub edition: String,
    pub book: usize,
    pub chapter: usize,
    #[serde(default)]
    pub verse: usize,
}
impl Position {
    pub fn same_chapter(&self, other: &Self) -> bool {
        self.edition == other.edition && self.book == other.book && self.chapter == other.chapter
    }
}
#[derive(Clone, Deserialize)]
pub struct Edition {
    pub id: String,
    pub language: String,
    pub books: usize,
    pub catholic: bool,
}
#[derive(Clone, Deserialize)]
pub struct Book {
    pub name: String,
    pub chapters: usize,
}
#[derive(Clone, Deserialize)]
pub struct Verse {
    pub verse: usize,
    pub label: String,
    pub text: String,
    pub annotation: Annotation,
}
#[derive(Clone, Deserialize)]
pub struct Hit {
    pub book: usize,
    pub chapter: usize,
    pub verse: usize,
    pub name: String,
    pub label: String,
    pub text: String,
    #[serde(default)]
    pub annotation: Option<Annotation>,
    #[serde(default)]
    pub organization: serde_json::Value,
    #[serde(skip)]
    pub organize_label: String,
}
#[derive(Clone, Debug)]
pub struct Span {
    pub prefix: i32,
    pub start: i32,
    pub end: i32,
}
#[derive(Clone, Debug)]
pub struct Selection {
    pub verse: usize,
    pub start: usize,
    pub length: usize,
    pub text: String,
}

/// GTK counts Unicode scalars, while the shared engine stores UTF-16 offsets.
pub fn utf16_offset(text: &str, chars: usize) -> usize {
    text.chars().take(chars).map(char::len_utf16).sum()
}
pub fn char_offset(text: &str, units: usize) -> Option<i32> {
    let mut total = 0;
    for (count, ch) in text.chars().enumerate() {
        if total == units {
            return i32::try_from(count).ok();
        }
        total += ch.len_utf16();
        if total > units {
            return None;
        }
    }
    (total == units).then(|| text.chars().count() as i32)
}
pub fn layout(verses: &[Verse]) -> (String, Vec<Span>) {
    let mut text = String::new();
    let mut spans = Vec::with_capacity(verses.len());
    let mut offset = 0;
    for v in verses {
        let prefix = format!(
            "{}{} {}\t",
            if v.annotation.bookmark { '★' } else { ' ' },
            if v.annotation.note.is_empty() {
                ' '
            } else {
                '✎'
            },
            v.label
        );
        let start = offset + prefix.chars().count() as i32;
        let end = start + v.text.chars().count() as i32;
        spans.push(Span {
            prefix: offset,
            start,
            end,
        });
        text.push_str(&prefix);
        text.push_str(&v.text);
        text.push('\n');
        offset = end + 1;
    }
    (text, spans)
}
pub fn selections(verses: &[Verse], spans: &[Span], a: i32, b: i32) -> Vec<Selection> {
    let (a, b) = (a.min(b), a.max(b));
    verses
        .iter()
        .zip(spans)
        .filter_map(|(verse, span)| {
            let start = a.max(span.start);
            let end = b.min(span.end);
            if start >= end {
                return None;
            }
            let first = (start - span.start) as usize;
            let count = (end - start) as usize;
            let text: String = verse.text.chars().skip(first).take(count).collect();
            Some(Selection {
                verse: verse.verse,
                start: utf16_offset(&verse.text, first),
                length: text.encode_utf16().count(),
                text,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_maps_across_verses_and_preserves_emoji_offsets() {
        let verses = vec![
            Verse {
                verse: 0,
                label: "4a".into(),
                text: "Luce 🌅 e pace".into(),
                annotation: Annotation {
                    note: "Nota 🌅".into(),
                    bookmark: true,
                    ..Default::default()
                },
            },
            Verse {
                verse: 1,
                label: "4b".into(),
                text: "Parola".into(),
                annotation: Annotation::default(),
            },
        ];
        let (text, spans) = layout(&verses);
        assert!(text.starts_with("★✎ "));
        let result = selections(&verses, &spans, spans[0].start + 5, spans[1].start + 3);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].start, 5);
        assert_eq!(result[0].text, "🌅 e pace");
        assert_eq!(result[0].length, 9);
        assert_eq!(result[1].text, "Par");
        assert!(selections(&verses, &spans, spans[0].prefix, spans[0].start).is_empty());
        assert_eq!(char_offset(&verses[0].text, 6), None);
        assert_eq!(char_offset(&verses[0].text, 7), Some(6));
    }
}
