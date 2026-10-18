use std::path::Path;

use dictos_core::DictError;
use dictos_dictionary::{import_jsonl, Dictionary, DictionaryBuilder};

fn fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/fixtures/mini-en.jsonl")
        .canonicalize()
        .expect("fixture mini-en.jsonl não encontrado")
}

fn build_from_fixture() -> (tempfile::TempDir, Dictionary) {
    let fixture = fixture_path();
    let entries: Vec<_> = import_jsonl(&fixture)
        .collect::<Result<_, _>>()
        .expect("fixture deve ser JSONL válido");
    assert!(!entries.is_empty());

    let dir = tempfile::tempdir().unwrap();
    DictionaryBuilder::build(entries, dir.path()).unwrap();
    let dict = Dictionary::open(dir.path()).unwrap();
    (dir, dict)
}

#[test]
fn importa_fixture_e_consulta() {
    let (_dir, dict) = build_from_fixture();

    let result = dict.lookup("apple").unwrap();
    assert_eq!(result.entry.word, "apple");
    assert_eq!(result.entry.lang, "en");
    assert!(result
        .entry
        .translations
        .iter()
        .any(|t| t.lang == "pt" && t.word == "maçã"));

    let result = dict.lookup("Serendipity").unwrap();
    assert!(!result.entry.senses.is_empty());
}

#[test]
fn consulta_desconhecida_no_fixture() {
    let (_dir, dict) = build_from_fixture();
    assert!(matches!(
        dict.lookup("xylophone"),
        Err(DictError::NotFound(_))
    ));
}

#[test]
fn sugestao_por_prefixo_no_fixture() {
    let (_dir, dict) = build_from_fixture();
    let hits = dict.lookup_prefix("li", 10);
    assert_eq!(hits, vec!["light"]);
}
