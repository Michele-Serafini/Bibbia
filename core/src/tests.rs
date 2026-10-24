use super::*;

#[test]
fn complete_corpus_navigation_and_search() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let books = engine.request(json!({"op":"books"})).unwrap();
    assert_eq!(books.as_array().unwrap().len(), 66);
    let chapter = engine
        .request(json!({"op":"chapter","book":0,"chapter":0}))
        .unwrap();
    assert_eq!(chapter.as_array().unwrap().len(), 31);
    assert!(
        chapter[0]["text"]
            .as_str()
            .unwrap()
            .contains("In the beginning")
    );
    let hits = engine
        .request(json!({"op":"search","query":"For God so loved the world"}))
        .unwrap();
    assert!(
        hits["hits"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["book"] == 42 && v["chapter"] == 2 && v["verse"] == 15)
    );
    let common = engine
        .request(json!({"op":"search","query":"the"}))
        .unwrap();
    assert_eq!(common["hits"].as_array().unwrap().len(), 200);
    assert_eq!(common["truncated"], true);
    assert!(
        engine
            .request(json!({"op":"chapter","book":66,"chapter":0}))
            .is_err()
    );
}

#[test]
fn personal_data_and_position_survive_restart_and_removal() {
    let dir = tempfile::tempdir().unwrap();
    let mark = json!({"highlight":"yellow","underline":true,"bookmark":true,"note":"Luce e speranza 🌅","ranges":[]});
    {
        let mut e = Engine::open(dir.path()).unwrap();
        e.request(json!({"op":"annotate","book":0,"chapter":0,"verse":2,"annotation":mark}))
            .unwrap();
        e.request(json!({"op":"settings","key":"position","value":{"edition":"KJV","book":42,"chapter":2}})).unwrap();
        e.request(json!({"op":"settings","key":"theme","value":"Dark"}))
            .unwrap();
    }
    let mut e = Engine::open(dir.path()).unwrap();
    let rows = e.request(json!({"op":"saved"})).unwrap();
    assert_eq!(rows[0]["annotation"], mark);
    let boot = e.request(json!({"op":"bootstrap"})).unwrap();
    assert_eq!(boot["settings"]["position"]["book"], 42);
    assert_eq!(boot["settings"]["theme"], "Dark");
    e.request(
        json!({"op":"annotate","book":0,"chapter":0,"verse":2,"annotation":Annotation::default()}),
    )
    .unwrap();
    assert_eq!(e.request(json!({"op":"saved"})).unwrap(), json!([]));
}

#[test]
fn import_is_validated_persistent_and_isolated_by_edition() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("italiano.json");
    std::fs::write(&source,r#"[{"name":"Genesi","abbrev":"gen","chapters":[["Nel principio Iddio creò il cielo e la terra."]]}]"#).unwrap();
    let req = json!({"op":"import","id":"Diodati locale","path":source});
    let mut e = Engine::open(dir.path()).unwrap();
    e.request(req.clone()).unwrap();
    assert!(e.request(req).is_err());
    e.request(
        json!({"op":"annotate","edition":"Diodati locale","book":0,"chapter":0,"verse":0,
        "annotation":{"highlight":"blue","underline":false,"bookmark":true,"note":"Nota"}}),
    )
    .unwrap();
    assert_eq!(e.request(json!({"op":"saved"})).unwrap(), json!([]));
    drop(e);
    let mut e = Engine::open(dir.path()).unwrap();
    let c = e
        .request(json!({"op":"chapter","edition":"Diodati locale","book":0,"chapter":0}))
        .unwrap();
    assert_eq!(c[0]["annotation"]["bookmark"], true);
    assert!(c[0]["text"].as_str().unwrap().contains("Iddio"));
    std::fs::write(&source, "[]").unwrap();
    assert!(
        e.request(json!({"op":"import","id":"vuota","path":source}))
            .is_err()
    );
}

#[test]
fn rejects_bad_inputs_without_mutating_data() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    for req in [
        json!({"op":"search","query":"a"}),
        json!({"op":"settings","key":"fontSize","value":100}),
        json!({"op":"settings","key":"theme","value":"invalid"}),
        json!({"op":"settings","key":"position","value":{"edition":"KJV","book":999,"chapter":0}}),
        json!({"op":"annotate","book":0,"chapter":0,"verse":999,"annotation":Annotation::default()}),
        json!({"op":"annotate","book":0,"chapter":0,"verse":0,"annotation":{"highlight":"red","underline":false,"bookmark":false,"note":""}}),
    ] {
        assert!(e.request(req).is_err());
    }
    assert_eq!(e.request(json!({"op":"saved"})).unwrap(), json!([]));
}

#[test]
fn c_abi_owns_utf8_responses_and_reports_errors() {
    unsafe {
        let result = bible_request(std::ptr::null());
        let parsed: Value = serde_json::from_str(CStr::from_ptr(result).to_str().unwrap()).unwrap();
        assert_eq!(parsed["ok"], false);
        bible_free(result);
        let dir = tempfile::tempdir().unwrap();
        let init = CString::new(json!({"op":"init","directory":dir.path()}).to_string()).unwrap();
        let result = bible_request(init.as_ptr());
        let parsed: Value = serde_json::from_str(CStr::from_ptr(result).to_str().unwrap()).unwrap();
        assert!(
            parsed["data"]["editions"]
                .as_array()
                .unwrap()
                .contains(&json!("KJV"))
        );
        assert_eq!(parsed["data"]["defaultEdition"], DEFAULT_EDITION);
        bible_free(result);
        let invalid = CString::new("{").unwrap();
        let result = bible_request(invalid.as_ptr());
        assert!(
            CStr::from_ptr(result)
                .to_str()
                .unwrap()
                .contains("\"ok\":false")
        );
        bible_free(result);
        let close = CString::new(r#"{"op":"close"}"#).unwrap();
        let result = bible_request(close.as_ptr());
        let parsed: Value = serde_json::from_str(CStr::from_ptr(result).to_str().unwrap()).unwrap();
        assert_eq!(parsed["ok"], true);
        bible_free(result);
        // In particular, Windows cannot remove a database whose connection is still open.
        std::fs::remove_file(dir.path().join("bible.sqlite3")).unwrap();
        bible_free(std::ptr::null_mut());
    }
}

#[test]
fn italian_corpus_is_complete_and_searchable() {
    let bible: Vec<Book> = serde_json::from_str(DIODATI).unwrap();
    assert_eq!(bible.len(), 66);
    assert_eq!(bible.iter().map(|b| b.chapters.len()).sum::<usize>(), 1189);
    assert_eq!(
        bible
            .iter()
            .flat_map(|b| &b.chapters)
            .map(Vec::len)
            .sum::<usize>(),
        31102
    );
    assert!(bible[0].chapters[0][0].contains("Iddio creò il cielo"));
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    let result = e
        .request(
            json!({"op":"search","edition":"Diodati 1649","query":"Iddio ha tanto amato il mondo"}),
        )
        .unwrap();
    assert!(
        result["hits"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["book"] == 42 && v["chapter"] == 2 && v["verse"] == 15)
    );
}

#[test]
fn verse_position_is_validated_and_legacy_positions_still_work() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    for verse in [json!(-1), json!(31), json!("1")] {
        assert!(e.request(json!({"op":"settings","key":"position","value":{"edition":"KJV","book":0,"chapter":0,"verse":verse}})).is_err());
    }
    e.request(
        json!({"op":"settings","key":"position","value":{"edition":"KJV","book":0,"chapter":0}}),
    )
    .unwrap();
    e.request(json!({"op":"settings","key":"position","value":{"edition":"KJV","book":0,"chapter":0,"verse":30}})).unwrap();
    drop(e);
    let mut e = Engine::open(dir.path()).unwrap();
    assert_eq!(
        e.request(json!({"op":"bootstrap"})).unwrap()["settings"]["position"]["verse"],
        30
    );
}

#[test]
fn catholic_edition_has_all_books_and_greek_additions() {
    let books: Vec<Book> = serde_json::from_str(MARTINI).unwrap();
    assert_eq!(books.len(), 73);
    for (id, chapters) in [
        ("TB", 14),
        ("JT", 16),
        ("WS", 19),
        ("SR", 51),
        ("BR", 6),
        ("M1", 16),
        ("M2", 15),
        ("ET", 16),
        ("DN", 14),
    ] {
        let book = books.iter().find(|b| b.abbrev == id).unwrap();
        assert_eq!(book.chapters.len(), chapters);
        assert!(
            book.chapters
                .iter()
                .all(|c| !c.is_empty() && c.iter().all(|v| !v.is_empty()))
        );
    }
    assert_eq!(books.iter().map(|b| b.chapters.len()).sum::<usize>(), 1334);
}

#[test]
fn phrase_marks_preserve_legacy_notes_and_survive_restart() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    // Raw old payload deliberately has no ranges, as in version 0.1.
    e.db.execute(
        "INSERT INTO annotations VALUES ('KJV',0,0,0,?)",
        [r#"{"highlight":"yellow","underline":false,"bookmark":true,"note":"Vecchia nota"}"#],
    )
    .unwrap();
    e.request(json!({"op":"mark","book":0,"chapter":0,"verse":0,"start":3,"length":9,"action":"underline"})).unwrap();
    e.request(
        json!({"op":"mark","book":0,"chapter":0,"verse":0,"start":3,"length":2,"action":"clear"}),
    )
    .unwrap();
    drop(e);
    let e = Engine::open(dir.path()).unwrap();
    let a = e.annotation("KJV", 0, 0, 0).unwrap();
    assert_eq!(a.note, "Vecchia nota");
    assert!(a.bookmark);
    assert!(a.highlight.is_none());
    assert!(
        a.ranges
            .iter()
            .any(|m| m.start == 0 && m.length == 3 && m.highlight.as_deref() == Some("yellow"))
    );
    assert!(
        a.ranges
            .iter()
            .any(|m| m.start == 5 && m.length == 7 && m.underline)
    );
    assert!(
        a.ranges
            .iter()
            .all(|m| m.start >= 5 || m.start + m.length <= 3)
    );
}

#[test]
fn utf16_selection_rejects_split_emoji_and_merges_adjacent_ranges() {
    let text = "A🌱 parola è luce";
    let mut a = Annotation::default();
    assert!(selection::apply_mark(text, &mut a, 1, 1, "underline").is_err());
    assert!(selection::apply_mark(text, &mut a, 2, 1, "underline").is_err());
    selection::apply_mark(text, &mut a, 1, 2, "underline").unwrap();
    selection::apply_mark(text, &mut a, 3, 7, "underline").unwrap();
    assert_eq!(a.ranges.len(), 1);
    assert_eq!(a.ranges[0].length, 9);
    selection::apply_mark(text, &mut a, 4, 6, "blue").unwrap();
    assert!(
        a.ranges
            .iter()
            .any(|m| m.start == 4 && m.highlight.as_deref() == Some("blue") && m.underline)
    );
    let before = a.clone();
    assert!(selection::apply_mark(text, &mut a, 99, 1, "clear").is_err());
    assert_eq!(a, before);
}

#[test]
fn interface_and_bible_language_preferences_are_independent() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    e.request(json!({"op":"settings","key":"language","value":"en"}))
        .unwrap();
    e.request(json!({"op":"settings","key":"bibleLanguage","value":"it"}))
        .unwrap();
    assert!(
        e.request(json!({"op":"settings","key":"language","value":"bogus"}))
            .is_err()
    );
    drop(e);
    let mut e = Engine::open(dir.path()).unwrap();
    let b = e.request(json!({"op":"bootstrap"})).unwrap();
    assert_eq!(b["settings"]["language"], "en");
    assert_eq!(b["settings"]["bibleLanguage"], "it");
    assert!(
        b["editionMetadata"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["catholic"] == true && e["language"] == "it" && e["books"] == 73)
    );
}
