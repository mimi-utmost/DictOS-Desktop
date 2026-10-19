use std::path::PathBuf;

use dictos_dictionary::wordnet::import_wordnet;
use dictos_dictionary::{Dictionary, DictionaryBuilder};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wordnet")
}

fn build_fixture() -> (tempfile::TempDir, Dictionary) {
    let entries = import_wordnet(&fixture_dir()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    DictionaryBuilder::build(entries, dir.path()).unwrap();
    let dict = Dictionary::open(dir.path()).unwrap();
    (dir, dict)
}

#[test]
fn importa_lemas_com_definicao_e_exemplos() {
    let (_dir, dict) = build_fixture();

    let result = dict.lookup("apple").unwrap();
    let entry = &result.entry;
    assert_eq!(entry.lang, "en");
    // O índice aponta para 2 synsets, mas só 1 existe no data.noun do fixture:
    // offsets sem gloss são ignorados em vez de abortar a importação.
    assert_eq!(entry.senses.len(), 1);
    assert_eq!(entry.senses[0].pos, "noun");
    assert_eq!(
        entry.senses[0].definition,
        "fruit with red or yellow or green skin and sweet to tart crisp whitish flesh"
    );

    let result = dict.lookup("serendipity").unwrap();
    assert_eq!(
        result.entry.senses[0].definition,
        "good luck in making unexpected and fortunate discoveries"
    );

    let result = dict.lookup("quickly").unwrap();
    assert_eq!(result.entry.senses[0].pos, "adverb");
    assert_eq!(result.entry.senses[0].examples, vec!["he works quickly"]);
}

#[test]
fn lema_em_duas_classes_gera_acepcoes_mescladas() {
    let (_dir, dict) = build_fixture();

    let result = dict.lookup("run").unwrap();
    let entry = &result.entry;
    assert_eq!(entry.senses.len(), 2);
    let poses: Vec<&str> = entry.senses.iter().map(|s| s.pos.as_str()).collect();
    assert!(poses.contains(&"noun"));
    assert!(poses.contains(&"verb"));

    let noun = entry.senses.iter().find(|s| s.pos == "noun").unwrap();
    assert_eq!(noun.examples.len(), 2);
    assert!(noun.examples[0].contains("Yankees"));

    let verb = entry.senses.iter().find(|s| s.pos == "verb").unwrap();
    assert!(verb.definition.starts_with("move fast by using one's feet"));
    assert_eq!(verb.examples.len(), 2);
}

#[test]
fn adjetivo_e_satelite_mapeiam_para_adjective() {
    let (_dir, dict) = build_fixture();
    let result = dict.lookup("ephemeral").unwrap();
    assert_eq!(result.entry.senses[0].pos, "adjective");
    assert_eq!(result.entry.senses[0].examples.len(), 2);
}

#[test]
fn letra_maiuscula_na_consulta() {
    let (_dir, dict) = build_fixture();
    assert!(dict.lookup("Apple").is_ok());
}
