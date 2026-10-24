use crate::{Annotation, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// UTF-16 offsets match Windows RichEdit's text positions, including emoji.
#[derive(Clone, Default, Deserialize, Serialize, PartialEq, Debug)]
pub struct TextMark {
    pub start: usize,
    pub length: usize,
    pub highlight: Option<String>,
    pub underline: bool,
}

pub fn valid_color(color: &Option<String>) -> bool {
    color
        .as_deref()
        .is_none_or(|c| ["yellow", "green", "blue", "pink"].contains(&c))
}

pub fn validate_range(text: &str, start: usize, length: usize) -> Result<()> {
    let end = start.checked_add(length).ok_or("Selezione non valida")?;
    if length == 0 {
        return Err("Seleziona una parola o una frase".into());
    }
    let mut offset = 0;
    let mut start_ok = start == 0;
    let mut end_ok = false;
    for ch in text.chars() {
        offset += ch.len_utf16();
        start_ok |= offset == start;
        end_ok |= offset == end;
        if offset >= end {
            break;
        }
    }
    if !start_ok || !end_ok {
        return Err("Selezione non valida".into());
    }
    Ok(())
}

pub fn apply_mark(
    text: &str,
    annotation: &mut Annotation,
    start: usize,
    length: usize,
    action: &str,
) -> Result<()> {
    validate_range(text, start, length)?;
    if !["underline", "clear", "yellow", "green", "blue", "pink"].contains(&action) {
        return Err("Azione non valida".into());
    }
    let total = text.encode_utf16().count();
    let end = start + length;
    let mut boundaries = BTreeSet::from([0, total, start, end]);
    for mark in &annotation.ranges {
        validate_range(text, mark.start, mark.length)?;
        boundaries.insert(mark.start);
        boundaries.insert(mark.start + mark.length);
    }
    let boundaries: Vec<_> = boundaries.into_iter().collect();
    let mut ranges: Vec<TextMark> = Vec::new();
    for pair in boundaries.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let mut style = TextMark {
            start: a,
            length: b - a,
            highlight: annotation.highlight.clone(),
            underline: annotation.underline,
        };
        for mark in &annotation.ranges {
            if a >= mark.start && b <= mark.start + mark.length {
                if mark.highlight.is_some() {
                    style.highlight = mark.highlight.clone();
                }
                style.underline |= mark.underline;
            }
        }
        if a >= start && b <= end {
            match action {
                "underline" => style.underline = true,
                "clear" => {
                    style.highlight = None;
                    style.underline = false;
                }
                color => style.highlight = Some(color.into()),
            }
        }
        if style.highlight.is_none() && !style.underline {
            continue;
        }
        if let Some(last) = ranges.last_mut()
            && last.start + last.length == a
            && last.highlight == style.highlight
            && last.underline == style.underline
        {
            last.length += style.length;
        } else {
            ranges.push(style);
        }
    }
    if ranges.len() > 512 {
        return Err("Troppe annotazioni nel versetto".into());
    }
    annotation.highlight = None;
    annotation.underline = false;
    annotation.ranges = ranges;
    Ok(())
}
