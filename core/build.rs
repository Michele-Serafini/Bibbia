use serde_json::{Value, json};
use std::{env, fs, path::PathBuf};

fn main() {
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let editions = [
        ("kjv", "KJV", "en", "protestant"),
        ("diodati", "Diodati 1649", "it", "protestant"),
        ("martini", "Martini 1781 (cattolica)", "it", "catholic"),
        ("extended", "Raccolta estesa (83 sezioni)", "en", "extended"),
        ("cei2008", "CEI 2008", "it", "catholic"),
    ];
    let mut generated = String::new();
    let mut count = 0;
    for (file, id, language, canon) in editions {
        let path = format!("data/{file}.json");
        println!("cargo:rerun-if-changed={path}");
        count += 1;
        let books: Vec<Value> = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let book_count = books.len();
        let mut catalog = Vec::new();
        let (mut text, mut lower) = (String::new(), String::new());
        let (mut offsets, mut lower_offsets) = (Vec::new(), Vec::new());
        let mut first = 0u32;
        for book in books {
            let mut chapters = Vec::new();
            for (ci, chapter) in book["chapters"].as_array().unwrap().iter().enumerate() {
                let verses = chapter.as_array().unwrap();
                let labels = book
                    .get("labels")
                    .and_then(|v| v.get(ci))
                    .cloned()
                    .unwrap_or(json!([]));
                chapters.push(json!({"first": first, "count": verses.len(), "labels": labels}));
                for verse in verses {
                    let value = verse.as_str().unwrap();
                    offsets.extend_from_slice(&u32::try_from(text.len()).unwrap().to_le_bytes());
                    lower_offsets
                        .extend_from_slice(&u32::try_from(lower.len()).unwrap().to_le_bytes());
                    text.push_str(value);
                    lower.push_str(&value.to_lowercase());
                    first += 1;
                }
            }
            catalog
                .push(json!({"name":book["name"], "abbrev":book["abbrev"], "chapters":chapters}));
        }
        offsets.extend_from_slice(&u32::try_from(text.len()).unwrap().to_le_bytes());
        lower_offsets.extend_from_slice(&u32::try_from(lower.len()).unwrap().to_le_bytes());
        for (suffix, data) in [
            ("text", text.as_bytes()),
            ("lower", lower.as_bytes()),
            ("offsets", &offsets),
            ("lower-offsets", &lower_offsets),
        ] {
            fs::write(out.join(format!("{file}-{suffix}.bin")), data).unwrap();
        }
        fs::write(
            out.join(format!("{file}-catalog.json")),
            serde_json::to_vec(&catalog).unwrap(),
        )
        .unwrap();
        generated.push_str(&format!("Builtin {{id:{id:?},book_count:{book_count},language:{language:?},canon:{canon:?},catalog_json:include_str!(concat!(env!(\"OUT_DIR\"),\"/{file}-catalog.json\")),catalog:std::sync::OnceLock::new(),text:include_str!(concat!(env!(\"OUT_DIR\"),\"/{file}-text.bin\")),lower:include_str!(concat!(env!(\"OUT_DIR\"),\"/{file}-lower.bin\")),offsets:include_bytes!(concat!(env!(\"OUT_DIR\"),\"/{file}-offsets.bin\")),lower_offsets:include_bytes!(concat!(env!(\"OUT_DIR\"),\"/{file}-lower-offsets.bin\"))}},\n"));
    }
    generated = format!("pub static BUILTINS: [Builtin;{count}] = [\n{generated}];\n");
    fs::write(out.join("corpora.rs"), generated).unwrap();
}
