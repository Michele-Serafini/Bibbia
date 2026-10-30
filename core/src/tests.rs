use super::*;

#[test]
fn compact_psalm_groups_preserve_every_verified_reference() {
    let data: Value = serde_json::from_str(include_str!("../data/comparison.json")).unwrap();
    for (file, edition) in [
        ("cei2008", "CEI 2008"),
        ("martini", "Martini 1781 (cattolica)"),
        ("diodati", "Diodati 1649"),
        ("kjv", "KJV"),
        ("extended", "Raccolta estesa (83 sezioni)"),
    ] {
        for (c, chapter) in data["psalms"][file].as_array().unwrap().iter().enumerate() {
            for (v, key) in chapter.as_array().unwrap().iter().enumerate() {
                let key = key.as_str().unwrap();
                let expected = if key.starts_with("unmapped:") {
                    None
                } else {
                    let parts: Vec<_> = key.split(':').collect();
                    Some(
                        parts[1].parse::<u16>().unwrap() * 256
                            + if parts[2] == "Title" {
                                0
                            } else {
                                parts[2].parse::<u16>().unwrap()
                            },
                    )
                };
                assert_eq!(
                    comparison::psalm_group(edition, c, v + 1),
                    expected,
                    "{edition} {c}:{v}"
                );
            }
        }
    }
}

#[test]
fn comparisons_use_book_identity_and_original_verse_references() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let john = engine
        .request(json!({"op":"compare","edition":"CEI 2008","other":"KJV","book":49,"chapter":2}))
        .unwrap();
    assert_eq!(john["status"], "aligned");
    let row = &john["rows"][15];
    assert_eq!(row["left"][0]["label"], "16");
    assert_eq!(row["right"][0]["book"], 42);
    assert!(
        row["right"][0]["text"]
            .as_str()
            .unwrap()
            .contains("God so loved")
    );
    let psalm = engine.request(json!({"op":"compare","edition":"CEI 2008","other":"Martini 1781 (cattolica)","book":22,"chapter":22})).unwrap();
    assert_eq!(psalm["rows"][0]["right"][0]["chapter"], 21);
    assert_eq!(psalm["rows"][0]["right"][0]["label"], "1");
    let joel = engine
        .request(json!({"op":"compare","edition":"CEI 2008","other":"KJV","book":35,"chapter":2}))
        .unwrap();
    assert_eq!(joel["rows"][0]["right"][0]["reference"], "Joel 2:28");
    let absent = engine
        .request(json!({"op":"compare","edition":"CEI 2008","other":"KJV","book":16,"chapter":0}))
        .unwrap();
    assert_eq!(absent["status"], "missingBook");
    assert!(
        absent["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["right"].as_array().unwrap().is_empty())
    );
    let esther = engine
        .request(json!({"op":"compare","edition":"CEI 2008","other":"KJV","book":18,"chapter":0}))
        .unwrap();
    assert!(
        esther["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["left"][0]["label"].as_str().unwrap().starts_with('G'))
            .all(|r| r["right"].as_array().unwrap().is_empty())
    );
}

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

#[test]
fn packed_text_and_casefold_indexes_preserve_every_original_verse() {
    for (id, source) in [
        ("KJV", KJV),
        ("Diodati 1649", DIODATI),
        (DEFAULT_EDITION, MARTINI),
        ("CEI 2008", include_str!("../data/cei2008.json")),
        (
            "Raccolta estesa (83 sezioni)",
            include_str!("../data/extended.json"),
        ),
    ] {
        let original: Vec<Book> = serde_json::from_str(source).unwrap();
        let packed = corpus::BUILTINS.iter().find(|b| b.id == id).unwrap();
        let mut count = 0;
        for (bi, book) in original.iter().enumerate() {
            let header = &packed.catalog()[bi];
            assert_eq!(header.name, book.name);
            assert_eq!(header.abbrev, book.abbrev);
            for (ci, chapter) in book.chapters.iter().enumerate() {
                let span = &header.chapters[ci];
                assert_eq!(span.count, chapter.len());
                if let Some(labels) = &book.labels {
                    assert_eq!(span.labels, labels[ci]);
                }
                for (vi, text) in chapter.iter().enumerate() {
                    assert_eq!(packed.text(span.first + vi), text);
                    assert_eq!(packed.lower(span.first + vi), text.to_lowercase());
                    count += 1;
                }
            }
        }
        assert!(count > 30_000);
    }
}

#[test]
fn imported_texts_are_lazy_and_only_one_edition_is_retained() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("text.json");
    std::fs::write(
        &path,
        r#"[{"name":"Local","abbrev":"loc","chapters":[["A small text."]]}]"#,
    )
    .unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    for id in ["First", "Second"] {
        engine
            .request(json!({"op":"import","id":id,"path":path}))
            .unwrap();
    }
    drop(engine);
    let mut engine = Engine::open(dir.path()).unwrap();
    assert!(engine.editions.values().all(|e| e.loaded.get().is_none()));
    for id in ["First", "Second", "KJV"] {
        engine
            .request(json!({"op":"chapter","edition":id,"book":0,"chapter":0}))
            .unwrap();
        assert!(
            engine
                .editions
                .iter()
                .all(|(name, e)| e.loaded.get().is_none() || name == id)
        );
    }
    engine
        .request(json!({"op":"compare","edition":"First","other":"Second","book":0,"chapter":0}))
        .unwrap();
    assert!(engine.editions["First"].loaded.get().is_some());
    assert!(engine.editions["Second"].loaded.get().is_some());
    engine
        .request(json!({"op":"books","edition":"First"}))
        .unwrap();
    assert!(engine.editions["First"].loaded.get().is_some());
    assert!(engine.editions["Second"].loaded.get().is_none());
    for edition in ["Second", "First"] {
        let id=engine.request(json!({"op":"reading","action":"create","title":"Calendar","edition":edition,"start":"2026-10-30","daily":1})).unwrap();
        engine.request(json!({"op":"reading","action":"calendar","id":id,"path":dir.path().join("plan.ics")})).unwrap();
        assert!(
            engine
                .editions
                .iter()
                .all(|(name, e)| e.loaded.get().is_none() || name == edition)
        );
    }
}

#[test]
fn unchanged_settings_do_not_write_again_and_chapter_reads_keep_marks() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = Engine::open(dir.path()).unwrap();
    let setting = json!({"op":"settings","key":"theme","value":"Dark"});
    engine.request(setting.clone()).unwrap();
    let before: i64 = engine
        .db
        .query_row("SELECT total_changes()", [], |r| r.get(0))
        .unwrap();
    engine.request(setting).unwrap();
    let after: i64 = engine
        .db
        .query_row("SELECT total_changes()", [], |r| r.get(0))
        .unwrap();
    assert_eq!(before, after);
    for verse in [0, 15, 30] {
        engine.request(json!({"op":"annotate","book":0,"chapter":0,"verse":verse,"annotation":{"highlight":null,"underline":false,"bookmark":true,"note":"Kept"}})).unwrap();
    }
    let chapter = engine
        .request(json!({"op":"chapter","book":0,"chapter":0}))
        .unwrap();
    assert_eq!(
        chapter
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["annotation"]["bookmark"] == true)
            .count(),
        3
    );
    assert_eq!(chapter[15]["label"], "16");
}

#[test]
fn extended_numbering_keeps_source_labels_and_does_not_invent_verses() {
    {
        let source = corpus::BUILTINS
            .iter()
            .find(|b| b.id == "Raccolta estesa (83 sezioni)")
            .unwrap();
        assert_eq!(source.catalog().len(), 83);
        for id in [
            "1ES", "2ES", "3MA", "4MA", "MAN", "PS2", "ESG", "DAG", "ENO", "JUB",
        ] {
            assert!(source.catalog().iter().any(|b| b.abbrev == id));
        }
        let sir = source.catalog().iter().find(|b| b.abbrev == "SIR").unwrap();
        assert!(!sir.chapters[0].labels.contains(&"5".into()));
        assert!(sir.chapters[0].labels.contains(&"6".into()));
        let enoch = source.catalog().iter().find(|b| b.abbrev == "ENO").unwrap();
        assert_eq!(enoch.chapters.len(), 108);
        let last = &enoch.chapters[107];
        assert_eq!(last.labels.last().unwrap(), "15");
        assert!(!source.text(last.first + last.count - 1).contains("Printed"));
        assert!(!source.text(enoch.chapters[0].first).contains(['\r', '\n']));
    }
}

#[test]
fn modern_cei_has_complete_catholic_text_and_distinct_esther_variants() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    let books = e
        .request(json!({"op":"books","edition":"CEI 2008"}))
        .unwrap();
    assert_eq!(books.as_array().unwrap().len(), 73);
    assert_eq!(
        books
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["chapters"].as_u64().unwrap())
            .sum::<u64>(),
        1328
    );
    for id in ["Tb", "Gdt", "Sap", "Sir", "Bar", "1Mac", "2Mac"] {
        assert!(
            books.as_array().unwrap().iter().any(|b| b["abbrev"] == id),
            "{id}"
        );
    }
    let first = e
        .request(json!({"op":"chapter","edition":"CEI 2008","book":0,"chapter":0}))
        .unwrap();
    assert_eq!(
        first[0]["text"],
        "In principio Dio creò il cielo e la terra."
    );
    assert_eq!(first[0]["label"], "1");
    let est = books
        .as_array()
        .unwrap()
        .iter()
        .position(|b| b["abbrev"] == "Est")
        .unwrap();
    let rows = e
        .request(json!({"op":"chapter","edition":"CEI 2008","book":est,"chapter":0}))
        .unwrap();
    for label in ["G1a", "G1", "E1"] {
        assert!(rows.as_array().unwrap().iter().any(|r| r["label"] == label));
    }
    let hits=e.request(json!({"op":"search","edition":"CEI 2008","query":"Dio infatti ha tanto amato il mondo"})).unwrap();
    assert!(
        hits["hits"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["label"] == "16")
    );
}

#[test]
fn malformed_import_labels_are_rejected_and_combined_labels_persist() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("labeled.json");
    let mut e = Engine::open(dir.path()).unwrap();
    for labels in [
        json!([]),
        json!([["1"]]),
        json!([["1", "1"]]),
        json!([["1", ""]]),
    ] {
        std::fs::write(
            &path,
            json!([{"name":"Test","abbrev":"T","chapters":[["One","Two"]],"labels":labels}])
                .to_string(),
        )
        .unwrap();
        assert!(
            e.request(json!({"op":"import","id":"Labeled","path":path}))
                .is_err()
        );
    }
    std::fs::write(
        &path,
        json!([{"name":"Test","abbrev":"T","chapters":[["One","Two"]],"labels":[["1a","2-3"]]}])
            .to_string(),
    )
    .unwrap();
    e.request(json!({"op":"import","id":"Labeled","path":path}))
        .unwrap();
    drop(e);
    let mut e = Engine::open(dir.path()).unwrap();
    let rows = e
        .request(json!({"op":"chapter","edition":"Labeled","book":0,"chapter":0}))
        .unwrap();
    assert_eq!(rows[1]["label"], "2-3");
}

#[test]
fn legacy_import_catalog_migrates_without_losing_notes_or_rewriting_text() {
    let dir = tempfile::tempdir().unwrap();
    let payload = r#"[{"name":"Old","abbrev":"O","chapters":[["Preserved text"]]}]"#;
    let db = Connection::open(dir.path().join("bible.sqlite3")).unwrap();
    db.execute_batch("CREATE TABLE editions (id TEXT PRIMARY KEY,payload TEXT NOT NULL);")
        .unwrap();
    db.execute(
        "INSERT INTO editions VALUES (?,?)",
        params!["Old import", payload],
    )
    .unwrap();
    drop(db);
    let mut e = Engine::open(dir.path()).unwrap();
    assert!(e.editions["Old import"].loaded.get().is_none());
    assert_eq!(e.editions["Old import"].book_count, 1);
    let stored: String =
        e.db.query_row(
            "SELECT payload FROM editions WHERE id='Old import'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(stored, payload);
    let rows = e
        .request(json!({"op":"chapter","edition":"Old import","book":0,"chapter":0}))
        .unwrap();
    assert_eq!(rows[0]["text"], "Preserved text");
    drop(e);
    let e = Engine::open(dir.path()).unwrap();
    assert!(e.editions["Old import"].loaded.get().is_none());
    let changes: i64 =
        e.db.query_row("SELECT total_changes()", [], |r| r.get(0))
            .unwrap();
    assert_eq!(changes, 0);
}

#[test]
fn newly_bundled_names_do_not_shadow_existing_imports_or_mix_their_notes() {
    let dir = tempfile::tempdir().unwrap();
    let e = Engine::open(dir.path()).unwrap();
    let payload = r#"[{"name":"My text","abbrev":"M","chapters":[["My original verse"]]}]"#;
    for id in ["CEI 2008", "CEI 2008 (importata)"] {
        e.db.execute(
            "INSERT INTO editions (id,payload,book_count) VALUES (?,?,1)",
            params![id, payload],
        )
        .unwrap();
    }
    let annotation = Annotation {
        bookmark: true,
        note: "My own note".into(),
        ..Default::default()
    };
    e.save_annotation("CEI 2008", 0, 0, 0, &annotation).unwrap();
    e.db.execute(
        "INSERT INTO settings VALUES ('position',?)",
        [json!({"edition":"CEI 2008","book":0,"chapter":0,"verse":0}).to_string()],
    )
    .unwrap();
    drop(e);
    let mut e = Engine::open(dir.path()).unwrap();
    let imported = "CEI 2008 (importata 2)";
    let boot = e.request(json!({"op":"bootstrap"})).unwrap();
    assert_eq!(boot["settings"]["position"]["edition"], imported);
    let own = e
        .request(json!({"op":"chapter","edition":imported,"book":0,"chapter":0}))
        .unwrap();
    assert_eq!(own[0]["text"], "My original verse");
    assert_eq!(own[0]["annotation"]["note"], "My own note");
    assert!(
        e.request(json!({"op":"saved","edition":"CEI 2008"}))
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        e.request(json!({"op":"books","edition":"CEI 2008"}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        73
    );
    drop(e);
    let mut e = Engine::open(dir.path()).unwrap();
    assert_eq!(
        e.request(json!({"op":"bootstrap"})).unwrap()["settings"]["position"]["edition"],
        imported
    );
}

#[test]
fn saved_pages_have_stable_order_and_keep_the_legacy_full_collection() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    for verse in [4, 0, 2] {
        e.request(json!({"op":"annotate","book":0,"chapter":0,"verse":verse,"annotation":{"highlight":null,"underline":false,"bookmark":true,"note":"Paged"}})).unwrap();
    }
    let first = e
        .request(json!({"op":"saved","limit":2,"offset":0}))
        .unwrap();
    assert_eq!(first[0]["verse"], 0);
    assert_eq!(first[1]["verse"], 2);
    let second = e
        .request(json!({"op":"saved","limit":2,"offset":2}))
        .unwrap();
    assert_eq!(second.as_array().unwrap().len(), 1);
    assert_eq!(second[0]["verse"], 4);
    assert_eq!(
        e.request(json!({"op":"saved"}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        3
    );
    for req in [
        json!({"op":"saved","limit":0}),
        json!({"op":"saved","limit":201}),
        json!({"op":"saved","offset":-1}),
    ] {
        assert!(e.request(req).is_err());
    }
}

#[test]
fn organized_notes_filter_search_rename_and_delete_without_losing_annotations() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    for verse in 0..4 {
        e.request(json!({"op":"annotate","edition":"KJV","book":0,"chapter":0,"verse":verse,"annotation":{"note":if verse%2==0{"PERCHÉ 🌅 gioia"}else{"Pace"},"bookmark":verse==3,"highlight":null,"underline":verse==1}})).unwrap();
    }
    e.request(json!({"op":"collection","kind":"folder","action":"create","name":"Vuota"}))
        .unwrap();
    for verse in [0, 2] {
        e.request(json!({"op":"organize","edition":"KJV","book":0,"chapter":0,"verse":verse,"folder":"Studio","tags":["speranza","amore","amore"]})).unwrap();
    }
    let result=e.request(json!({"op":"saved","edition":"KJV","query":"perché 🌅","folder":"Studio","tag":"amore","kind":"note","limit":1,"offset":1})).unwrap();
    assert_eq!(result[0]["verse"], 2);
    assert_eq!(
        result[0]["organization"]["tags"],
        json!(["amore", "speranza"])
    );
    assert_eq!(
        e.request(json!({"op":"saved","edition":"KJV","query":"created","kind":"all"}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        e.request(json!({"op":"saved","edition":"KJV","kind":"marked"}))
            .unwrap()[0]["verse"],
        1
    );
    assert_eq!(
        e.request(json!({"op":"saved","edition":"KJV","folder":""}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(
        e.request(json!({"op":"saved","edition":"KJV","query":"%' OR 1=1 --"}))
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(e.request(json!({"op":"collection","kind":"folder","action":"rename","name":"Studio","newName":"Vuota"})).is_err());
    e.request(json!({"op":"collection","kind":"folder","action":"rename","name":"Studio","newName":"Vangelo"})).unwrap();
    e.request(
        json!({"op":"collection","kind":"tag","action":"rename","name":"amore","newName":"carità"}),
    )
    .unwrap();
    e.request(json!({"op":"mark","edition":"KJV","book":0,"chapter":0,"verse":0,"start":0,"length":2,"action":"yellow"})).unwrap();
    drop(e);
    let mut e = Engine::open(dir.path()).unwrap();
    let rows = e
        .request(json!({"op":"saved","edition":"KJV","folder":"Vangelo","tag":"carità"}))
        .unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 2);
    assert_eq!(rows[0]["annotation"]["note"], "PERCHÉ 🌅 gioia");
    e.request(json!({"op":"collection","kind":"folder","action":"delete","name":"Vangelo"}))
        .unwrap();
    e.request(json!({"op":"collection","kind":"tag","action":"delete","name":"carità"}))
        .unwrap();
    let rows = e.request(json!({"op":"saved","edition":"KJV"})).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 4);
    assert_eq!(rows[0]["organization"]["folder"], "");
    assert_eq!(rows[0]["organization"]["tags"], json!(["speranza"]));
    e.request(json!({"op":"annotate","edition":"KJV","book":0,"chapter":0,"verse":0,"annotation":Annotation::default()})).unwrap();
    assert_eq!(
        e.db.query_row(
            "SELECT count(*) FROM note_organization WHERE verse=0",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert!(e.request(json!({"op":"organize","edition":"KJV","book":0,"chapter":0,"verse":99,"folder":"Invalid","tags":[]})).is_err());
}

#[test]
fn reading_calendar_progress_and_colored_notes_survive_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    let id=e.request(json!({"op":"reading","action":"create","edition":"CEI 2008","title":"Quaresima 🌅","start":"2024-02-28","daily":2,"book":0})).unwrap();
    let mut detail=e.request(json!({"op":"reading","action":"detail","id":id,"from":"2024-02-28","selected":"2024-02-29"})).unwrap();
    assert_eq!(detail["total"], 50);
    assert_eq!(detail["duration"], 25);
    assert_eq!(detail["chapters"][0]["chapter"], 2);
    assert_eq!(detail["days"].as_array().unwrap().len(), 42);
    for index in 0..2 {
        e.request(
            json!({"op":"reading","action":"complete","id":id,"index":index,"complete":true}),
        )
        .unwrap();
    }
    e.request(json!({"op":"reading","action":"complete","id":id,"index":0,"complete":true}))
        .unwrap();
    detail=e.request(json!({"op":"reading","action":"detail","id":id,"from":"2024-02-28","selected":"2024-02-27"})).unwrap();
    assert_eq!(detail["completed"], 2);
    assert_eq!(detail["days"][0]["done"], 2);
    assert!(detail["chapters"].as_array().unwrap().is_empty());
    assert_eq!(
        e.request(json!({"op":"reading","action":"resume","id":id}))
            .unwrap()["chapter"],
        2
    );
    let second=e.request(json!({"op":"reading","action":"create","edition":"CEI 2008","title":"Altro piano","start":"2024-02-28","daily":1,"book":0})).unwrap();
    e.request(json!({"op":"settings","key":"position","value":{"edition":"CEI 2008","book":0,"chapter":2,"verse":4}})).unwrap();
    assert_eq!(e.request(json!({"op":"reading","action":"detail","id":second,"from":"2024-02-28","selected":"2024-02-28"})).unwrap()["resume"]["chapter"],0);
    e.request(json!({"op":"note","edition":"CEI 2008","book":0,"chapter":2,"verse":4,"text":"Speranza 🌅","color":"blue","style":"sticky"})).unwrap();
    e.request(json!({"op":"organize","edition":"CEI 2008","book":0,"chapter":2,"verse":4,"folder":"Studio","tags":["speranza"]})).unwrap();
    e.request(json!({"op":"mark","edition":"CEI 2008","book":0,"chapter":2,"verse":4,"start":0,"length":2,"action":"underline"})).unwrap();
    let mut legacy = e
        .request(json!({"op":"chapter","edition":"CEI 2008","book":0,"chapter":2}))
        .unwrap()[4]["annotation"]
        .clone();
    legacy.as_object_mut().unwrap().remove("noteColor");
    legacy.as_object_mut().unwrap().remove("noteStyle");
    e.request(json!({"op":"annotate","edition":"CEI 2008","book":0,"chapter":2,"verse":4,"annotation":legacy})).unwrap();
    drop(e);
    let mut e = Engine::open(dir.path()).unwrap();
    assert_eq!(
        e.request(json!({"op":"reading","action":"resume","id":id}))
            .unwrap()["verse"],
        4
    );
    let rows = e
        .request(json!({"op":"saved","edition":"CEI 2008","query":"SPERANZA","tag":"speranza"}))
        .unwrap();
    assert_eq!(rows[0]["annotation"]["noteColor"], "blue");
    assert_eq!(rows[0]["annotation"]["noteStyle"], "sticky");
    assert_eq!(rows[0]["organization"]["folder"], "Studio");
    assert!(
        e.request(json!({"op":"reading","action":"complete","id":id,"index":50,"complete":true}))
            .is_err()
    );
    assert!(e.request(json!({"op":"reading","action":"create","edition":"KJV","title":"Bad","start":"2023-02-29","daily":2})).is_err());
    assert!(e.request(json!({"op":"note","edition":"KJV","book":0,"chapter":0,"verse":0,"text":"Invalid","color":"red","style":"sticky"})).is_err());
    e.request(json!({"op":"reading","action":"complete","id":id,"index":0,"complete":false}))
        .unwrap();
    e.request(json!({"op":"reading","action":"delete","id":id}))
        .unwrap();
    assert_eq!(
        e.db.query_row("SELECT count(*) FROM reading_progress", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn portable_backup_restore_is_atomic_and_keeps_a_safety_copy() {
    let source = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let backup = output.path().join("Appunti perché.bibbia-backup");
    let mut e = Engine::open(source.path()).unwrap();
    e.request(json!({"op":"note","edition":"KJV","book":0,"chapter":0,"verse":0,"text":"Perché Ω 🌅\n# nota <script> &copy;","color":"blue","style":"sticky"})).unwrap();
    e.request(json!({"op":"organize","edition":"KJV","book":0,"chapter":0,"verse":0,"folder":"Studio","tags":["speranza"]})).unwrap();
    e.request(json!({"op":"mark","edition":"KJV","book":0,"chapter":0,"verse":0,"start":0,"length":2,"action":"underline"})).unwrap();
    e.request(json!({"op":"reading","action":"create","edition":"KJV","title":"Lettura","start":"2026-10-29","daily":2,"book":0})).unwrap();
    e.request(json!({"op":"reading","action":"complete","id":1,"index":0,"complete":true}))
        .unwrap();
    e.request(json!({"op":"transfer","action":"backup","path":backup}))
        .unwrap();
    let markdown = output.path().join("note.md");
    e.request(json!({"op":"export","format":"markdown","path":markdown}))
        .unwrap();
    let md = std::fs::read_to_string(&markdown).unwrap();
    assert!(
        md.contains("Perché Ω 🌅")
            && md.contains("\\# nota \\<script\\>")
            && md.contains("\\&copy;")
            && md.contains("Studio")
            && md.contains("Post\\-it")
    );
    assert!(
        e.request(
            json!({"op":"export","format":"markdown","path":source.path().join("bible.sqlite3")})
        )
        .is_err()
    );
    assert!(e.request(json!({"op":"transfer","action":"check_destination","path":source.path().join("BIBLE.SQLITE3")})).is_err());
    let mut destination = Engine::open(target.path()).unwrap();
    destination.request(json!({"op":"note","edition":"KJV","book":0,"chapter":0,"verse":1,"text":"Originale","color":"yellow","style":"plain"})).unwrap();
    std::fs::write(target.path().join("reader.json"), "stale").unwrap();
    assert_eq!(
        destination
            .request(json!({"op":"transfer","action":"inspect","path":backup}))
            .unwrap()["annotations"],
        1
    );
    destination
        .request(json!({"op":"transfer","action":"restore","path":backup}))
        .unwrap();
    assert_eq!(
        destination
            .request(json!({"op":"saved","edition":"KJV"}))
            .unwrap()[0]["annotation"]["note"],
        "Originale"
    );
    drop(destination);
    let mut destination = Engine::open(target.path()).unwrap();
    let rows = destination
        .request(json!({"op":"saved","edition":"KJV"}))
        .unwrap();
    assert_eq!(rows[0]["annotation"]["noteColor"], "blue");
    assert_eq!(rows[0]["organization"]["folder"], "Studio");
    assert_eq!(rows[0]["annotation"]["ranges"][0]["length"], 2);
    assert_eq!(destination.request(json!({"op":"reading","action":"detail","id":1,"from":"2026-10-29","selected":"2026-10-29"})).unwrap()["completed"],1);
    assert!(!target.path().join("reader.json").exists());
    assert!(!target.path().join("pending-restore.sqlite3").exists());
    let safety = std::fs::read_dir(target.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "bibbia-backup"))
        .unwrap();
    let (old, _) = transfer::test_inspect(&safety).unwrap();
    assert!(
        old.query_row("SELECT payload FROM annotations", [], |r| r
            .get::<_, String>(0))
            .unwrap()
            .contains("Originale")
    );
    std::fs::write(&backup, b"not a backup").unwrap();
    assert!(
        destination
            .request(json!({"op":"transfer","action":"restore","path":backup}))
            .is_err()
    );
    assert_eq!(
        destination
            .request(json!({"op":"saved","edition":"KJV"}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn reading_appearance_is_validated_persistent_and_portable() {
    let dir = tempfile::tempdir().unwrap();
    let archive_dir = tempfile::tempdir().unwrap();
    let archive = archive_dir.path().join("reading.bibbia-backup");
    let mut engine = Engine::open(dir.path()).unwrap();
    for (key, value) in [
        ("theme", json!("Sepia")),
        ("readingMargin", json!(64)),
        ("focusMode", json!(true)),
    ] {
        engine
            .request(json!({"op":"settings","key":key,"value":value}))
            .unwrap();
    }
    for (key, value) in [
        ("readingMargin", json!(15)),
        ("readingMargin", json!(97)),
        ("readingMargin", json!(24.5)),
        ("readingMargin", json!("24")),
        ("focusMode", json!(1)),
        ("focusMode", json!("true")),
    ] {
        assert!(
            engine
                .request(json!({"op":"settings","key":key,"value":value}))
                .is_err()
        );
    }
    engine
        .request(json!({"op":"transfer","action":"backup","path":archive}))
        .unwrap();
    engine
        .request(json!({"op":"settings","key":"theme","value":"Dark"}))
        .unwrap();
    engine
        .request(json!({"op":"transfer","action":"restore","path":archive}))
        .unwrap();
    drop(engine);
    let mut engine = Engine::open(dir.path()).unwrap();
    let boot = engine.request(json!({"op":"bootstrap"})).unwrap();
    assert_eq!(boot["settings"]["theme"], "Sepia");
    assert_eq!(boot["settings"]["readingMargin"], 64);
    assert_eq!(boot["settings"]["focusMode"], true);
}

#[test]
fn selective_annotation_removal_preserves_other_data_and_rolls_back_failures() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    e.request(json!({"op":"note","edition":"KJV","book":0,"chapter":0,"verse":0,"text":"Luce 🌅","color":"pink","style":"sticky"})).unwrap();
    e.request(json!({"op":"mark","edition":"KJV","book":0,"chapter":0,"verse":0,"start":0,"length":2,"action":"underline"})).unwrap();
    e.request(json!({"op":"bookmark","edition":"KJV","book":0,"chapter":0,"verse":0}))
        .unwrap();
    e.request(json!({"op":"organize","edition":"KJV","book":0,"chapter":0,"verse":0,"folder":"Studio","tags":["luce"]})).unwrap();
    let a=e.request(json!({"op":"delete_annotation","kind":"note","edition":"KJV","book":0,"chapter":0,"verse":0})).unwrap();
    assert_eq!(a["note"], "");
    assert_eq!(a["bookmark"], true);
    assert_eq!(a["ranges"][0]["length"], 2);
    assert!(a.get("noteColor").is_none());
    e.db.execute_batch("CREATE TRIGGER deny_remove BEFORE DELETE ON annotations BEGIN SELECT RAISE(ABORT,'keep data'); END;").unwrap();
    assert!(e.request(json!({"op":"delete_annotation","kind":"all","edition":"KJV","book":0,"chapter":0,"verse":0})).is_err());
    let row = e.request(json!({"op":"saved","edition":"KJV"})).unwrap();
    assert_eq!(row[0]["organization"]["folder"], "Studio");
    assert_eq!(row[0]["annotation"], a);
    e.db.execute_batch("DROP TRIGGER deny_remove;").unwrap();
    assert!(e.request(json!({"op":"delete_annotation","kind":"invalid","edition":"KJV","book":0,"chapter":0,"verse":0})).is_err());
    e.request(json!({"op":"delete_annotation","kind":"marks","edition":"KJV","book":0,"chapter":0,"verse":0})).unwrap();
    let a = e
        .request(json!({"op":"bookmark","edition":"KJV","book":0,"chapter":0,"verse":0}))
        .unwrap();
    assert_eq!(a["bookmark"], false);
    assert_eq!(
        e.request(json!({"op":"saved","edition":"KJV"})).unwrap(),
        json!([])
    );
    assert_eq!(
        e.db.query_row("SELECT count(*) FROM note_organization", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn calendar_snapshots_have_stable_ids_utf8_folding_and_civil_dates() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    let path = dir.path().join("Piano Ω.ics");
    let id=e.request(json!({"op":"reading","action":"create","edition":"KJV","book":0,"title":"Lettura 🌅 Ω, speranza; \\ domani\nBEGIN:VEVENT","start":"2028-02-28","daily":1})).unwrap();
    assert_eq!(
        e.request(json!({"op":"reading","action":"calendar","id":id,"path":path,"language":"it"}))
            .unwrap()["events"],
        50
    );
    let bytes = std::fs::read(&path).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(text.ends_with("END:VCALENDAR\r\n"));
    assert!(text.split("\r\n").all(|line| line.len() <= 75));
    let unfolded = text.replace("\r\n ", "");
    assert_eq!(
        unfolded
            .split("\r\n")
            .filter(|line| *line == "BEGIN:VEVENT")
            .count(),
        50
    );
    for date in ["20280228", "20280229", "20280301", "20280326", "20280327"] {
        assert!(unfolded.contains(&format!("DTSTART;VALUE=DATE:{date}\r\n")));
    }
    assert!(unfolded.contains("\\, speranza\\;") && unfolded.contains("\\nBEGIN:VEVENT"));
    let ids: Vec<_> = unfolded
        .lines()
        .filter(|l| l.starts_with("UID:"))
        .map(str::to_owned)
        .collect();
    e.request(json!({"op":"reading","action":"complete","id":id,"index":0,"complete":true}))
        .unwrap();
    e.request(json!({"op":"reading","action":"calendar","id":id,"path":path,"language":"en"}))
        .unwrap();
    let second = std::fs::read_to_string(&path).unwrap().replace("\r\n ", "");
    assert_eq!(
        ids,
        second
            .lines()
            .filter(|l| l.starts_with("UID:"))
            .collect::<Vec<_>>()
    );
    assert!(second.contains("Completed: 1/1"));
    assert!(e.request(json!({"op":"reading","action":"calendar","id":id,"path":dir.path().join("bible.sqlite3")})).is_err());
    assert!(
        e.request(json!({"op":"reading","action":"calendar","id":99999,"path":path}))
            .is_err()
    );
    assert_eq!(
        std::fs::read_to_string(path).unwrap().replace("\r\n ", ""),
        second
    );
}
