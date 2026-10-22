//! Testes do lookup multi-dicionário (merge de acepções e traduções).

use dictos_core::{DictionaryEntry, Sense, Translation};
use dictos_dictionary::{lookup_merged, Dictionary, DictionaryBuilder};

fn entry(word: &str, senses: Vec<(&str, &str)>, translations: Vec<(&str, &str)>) -> DictionaryEntry {
    DictionaryEntry {
        word: word.into(),
        lang: "en".into(),
        senses: senses
            .into_iter()
            .map(|(pos, definition)| Sense {
                pos: pos.into(),
                definition: definition.into(),
                examples: vec![],
            })
            .collect(),
        translations: translations
            .into_iter()
            .map(|(lang, word)| Translation {
                lang: lang.into(),
                word: word.into(),
            })
            .collect(),
    }
}

#[test]
fn merge_acepcoes_de_um_e_traducoes_de_outro() {
    let dir = tempfile::tempdir().unwrap();
    let dir_a = dir.path().join("en");
    let dir_b = dir.path().join("en-pt");

    DictionaryBuilder::build(
        vec![entry("apple", vec![("noun", "A round fruit.")], vec![])],
        &dir_a,
    )
    .unwrap();
    DictionaryBuilder::build(
        vec![entry("apple", vec![], vec![("pt", "maçã")])],
        &dir_b,
    )
    .unwrap();

    let a = Dictionary::open(&dir_a).unwrap();
    let b = Dictionary::open(&dir_b).unwrap();
    let result = lookup_merged(&[&a, &b], "apple").unwrap();

    assert_eq!(result.entry.senses.len(), 1);
    assert_eq!(result.entry.senses[0].definition, "A round fruit.");
    assert_eq!(
        result.entry.translations,
        vec![Translation {
            lang: "pt".into(),
            word: "maçã".into()
        }]
    );
}

#[test]
fn merge_nao_duplica_e_notfound_so_quando_ausente_em_todos() {
    let dir = tempfile::tempdir().unwrap();
    let dir_a = dir.path().join("a");
    let dir_b = dir.path().join("b");

    DictionaryBuilder::build(vec![entry("run", vec![], vec![("pt", "correr")])], &dir_a).unwrap();
    DictionaryBuilder::build(vec![entry("run", vec![], vec![("pt", "correr"), ("pt", "executar")])], &dir_b).unwrap();

    let a = Dictionary::open(&dir_a).unwrap();
    let b = Dictionary::open(&dir_b).unwrap();

    let result = lookup_merged(&[&a, &b], "run").unwrap();
    assert_eq!(result.entry.translations.len(), 2);

    let err = lookup_merged(&[&a, &b], "inexistente").unwrap_err();
    assert!(matches!(err, dictos_core::DictError::NotFound(_)));
}
