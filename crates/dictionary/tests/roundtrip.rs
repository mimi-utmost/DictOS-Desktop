use std::fs;
use std::io::Write;

use dictos_core::{DictError, DictionaryEntry, Sense, Translation};
use dictos_dictionary::{import_jsonl, Dictionary, DictionaryBuilder};

fn entry(word: &str, senses: Vec<Sense>, translations: Vec<Translation>) -> DictionaryEntry {
    DictionaryEntry {
        word: word.to_string(),
        lang: "en".to_string(),
        senses,
        translations,
    }
}

fn sense(pos: &str, def: &str) -> Sense {
    Sense {
        pos: pos.to_string(),
        definition: def.to_string(),
        examples: vec![],
    }
}

fn translation(lang: &str, word: &str) -> Translation {
    Translation {
        lang: lang.to_string(),
        word: word.to_string(),
    }
}

fn build(entries: Vec<DictionaryEntry>) -> (tempfile::TempDir, Dictionary) {
    let dir = tempfile::tempdir().unwrap();
    DictionaryBuilder::build(entries, dir.path()).unwrap();
    let dict = Dictionary::open(dir.path()).unwrap();
    (dir, dict)
}

#[test]
fn roundtrip_builder_reader() {
    let entries = vec![
        entry(
            "apple",
            vec![sense("noun", "A round fruit with red or green skin.")],
            vec![translation("pt", "maçã")],
        ),
        entry("run", vec![sense("verb", "To move swiftly on foot.")], vec![]),
    ];
    let (_dir, dict) = build(entries);

    let result = dict.lookup("apple").unwrap();
    assert!(!result.diacritic_fallback);
    assert_eq!(result.entry.word, "apple");
    assert_eq!(result.entry.senses[0].pos, "noun");
    assert_eq!(result.entry.translations[0].word, "maçã");

    let result = dict.lookup("  RUN  ").unwrap();
    assert_eq!(result.entry.word, "run");
}

#[test]
fn lookup_fallback_sem_diacriticos() {
    let (_dir, dict) = build(vec![DictionaryEntry {
        word: "coração".to_string(),
        lang: "pt".to_string(),
        senses: vec![sense("noun", "Órgão muscular que bombeia o sangue.")],
        translations: vec![translation("en", "heart")],
    }]);

    let result = dict.lookup("coração").unwrap();
    assert!(!result.diacritic_fallback);

    let result = dict.lookup("coracao").unwrap();
    assert!(result.diacritic_fallback);
    assert_eq!(result.entry.word, "coração");
}

#[test]
fn lookup_prefixo() {
    let (_dir, dict) = build(vec![
        entry("apple", vec![], vec![]),
        entry("application", vec![], vec![]),
        entry("apply", vec![], vec![]),
        entry("banana", vec![], vec![]),
    ]);

    let hits = dict.lookup_prefix("app", 10);
    assert_eq!(hits, vec!["apple", "application", "apply"]);

    let hits = dict.lookup_prefix("app", 2);
    assert_eq!(hits.len(), 2);

    assert!(dict.lookup_prefix("zzz", 10).is_empty());
}

#[test]
fn palavra_desconhecida_retorna_not_found() {
    let (_dir, dict) = build(vec![entry("apple", vec![], vec![])]);

    match dict.lookup("nonexistent") {
        Err(DictError::NotFound(word)) => assert_eq!(word, "nonexistent"),
        other => panic!("esperava NotFound, veio {other:?}"),
    }
}

#[test]
fn duplicados_sao_mesclados() {
    let entries = vec![
        entry(
            "run",
            vec![sense("verb", "To move swiftly.")],
            vec![translation("pt", "correr")],
        ),
        entry(
            "Run",
            vec![sense("noun", "An act of running.")],
            vec![
                translation("pt", "correr"),
                translation("pt", "corrida"),
            ],
        ),
    ];
    let (_dir, dict) = build(entries);

    let result = dict.lookup("run").unwrap();
    assert_eq!(result.entry.senses.len(), 2);
    assert_eq!(result.entry.translations.len(), 2);
}

#[test]
fn jsonl_malformado_erro_com_linha() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.jsonl");
    let mut file = fs::File::create(&path).unwrap();
    writeln!(
        file,
        r#"{{"word":"ok","lang":"en","senses":[],"translations":[]}}"#
    )
    .unwrap();
    writeln!(file, "{{isto não é json").unwrap();
    writeln!(file).unwrap();
    writeln!(
        file,
        r#"{{"word":"also-ok","lang":"en","senses":[],"translations":[]}}"#
    )
    .unwrap();
    drop(file);

    let results: Vec<_> = import_jsonl(&path).collect();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].as_ref().unwrap().word, "ok");
    match &results[1] {
        Err(DictError::Format(msg)) => assert!(msg.starts_with("linha 2:"), "msg: {msg}"),
        other => panic!("esperava Format, veio {other:?}"),
    }
    assert_eq!(results[2].as_ref().unwrap().word, "also-ok");
}
