//! Importador de traduções do Kaikki/Wiktionary (CC BY-SA 3.0).
//!
//! Lê o JSONL "raw" do enwiktionary em streaming (aceita `.jsonl` puro ou
//! `.jsonl.gz`), filtra apenas entradas com tradução para o idioma-alvo e
//! gera entradas **somente de tradução** (definições en–en já vêm do
//! WordNet). O dado resultante é CC BY-SA: opcional, nunca empacotado, e o
//! diretório de saída recebe um arquivo `NOTICES` com a atribuição.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use dictos_core::{DictError, DictionaryEntry, Translation};
use serde::Deserialize;

/// Texto de atribuição exigido pela CC BY-SA 3.0, gravado como `NOTICES`
/// no diretório do dicionário gerado.
pub const KAIKKI_NOTICES: &str = "\
Este dicionário contém traduções derivadas do Wiktionary em inglês,
extraídas por wiktextract e distribuídas por kaikki.org.

Fonte: https://kaikki.org/dictionary/rawdata.html
Licença dos dados: Creative Commons Attribution-ShareAlike 3.0 Unported
  https://creativecommons.org/licenses/by-sa/3.0/
Os dados derivados (este dicionário) permanecem sob CC BY-SA 3.0.
Atribuição: Wiktionary contributors / kaikki.org (Tatu Ylonen).
";

#[derive(Deserialize)]
struct KaikkiLine {
    word: String,
    #[serde(default)]
    translations: Vec<KaikkiTranslation>,
}

#[derive(Deserialize)]
struct KaikkiTranslation {
    lang_code: Option<String>,
    word: Option<String>,
}

/// Estatísticas do import, para o relatório do CLI.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ImportStats {
    pub lines_read: u64,
    pub lines_ignored: u64,
    pub entries: u64,
    pub translations: u64,
}

/// Extrai traduções `target_lang` (ex.: "pt") de um JSONL Kaikki.
///
/// Retorna as entradas (uma por headword, traduções deduplicadas e
/// ordenadas) e estatísticas. Linhas malformadas são contadas e ignoradas —
/// num arquivo de 24 GB, exigir 100% de linhas válidas inviabiliza o import.
pub fn import_kaikki_translations(
    input: &Path,
    target_lang: &str,
) -> Result<(Vec<DictionaryEntry>, ImportStats), DictError> {
    let file = File::open(input).map_err(DictError::Io)?;
    let reader: Box<dyn BufRead> = if input.extension().is_some_and(|ext| ext == "gz") {
        Box::new(BufReader::new(flate2::read::GzDecoder::new(file)))
    } else {
        Box::new(BufReader::new(file))
    };

    let mut stats = ImportStats::default();
    let mut by_word: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for line in reader.lines() {
        let line = line.map_err(DictError::Io)?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        stats.lines_read += 1;

        let parsed = serde_json::from_str::<KaikkiLine>(line);
        let Ok(entry) = parsed else {
            stats.lines_ignored += 1;
            continue;
        };

        let pt: Vec<String> = entry
            .translations
            .iter()
            .filter(|t| t.lang_code.as_deref() == Some(target_lang))
            .filter_map(|t| t.word.clone())
            .map(|w| w.trim().to_string())
            .filter(|w| !w.is_empty())
            .collect();
        if pt.is_empty() {
            stats.lines_ignored += 1;
            continue;
        }
        stats.translations += pt.len() as u64;
        by_word
            .entry(entry.word.trim().to_lowercase())
            .or_default()
            .extend(pt);
    }

    let entries: Vec<DictionaryEntry> = by_word
        .into_iter()
        .map(|(word, translations)| DictionaryEntry {
            word,
            lang: "en".into(),
            senses: Vec::new(),
            translations: translations
                .into_iter()
                .map(|word| Translation {
                    lang: target_lang.to_string(),
                    word,
                })
                .collect(),
        })
        .collect();
    stats.entries = entries.len() as u64;

    Ok((entries, stats))
}

/// Grava o arquivo `NOTICES` de atribuição CC BY-SA no diretório de saída.
pub fn write_kaikki_notices(out_dir: &Path) -> Result<(), DictError> {
    std::fs::write(out_dir.join("NOTICES"), KAIKKI_NOTICES).map_err(DictError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const FIXTURE: &str = concat!(
        r#"{"word":"apple","lang_code":"en","pos":"noun","translations":[{"lang_code":"pt","lang":"Portuguese","word":"maçã"},{"lang_code":"de","lang":"German","word":"Apfel"}]}"#,
        "\n",
        r#"{"word":"apple","lang_code":"en","pos":"verb","translations":[{"lang_code":"pt","lang":"Portuguese","word":"maçã"}]}"#,
        "\n",
        r#"{"word":"run","lang_code":"en","pos":"verb","translations":[{"lang_code":"pt","lang":"Portuguese","word":"correr"},{"lang_code":"pt","lang":"Portuguese","word":"executar"}]}"#,
        "\n",
        r#"{"word":"no-translations","lang_code":"en","pos":"noun"}"#,
        "\n",
        "linha inválida que não é JSON\n",
    );

    fn write_fixture(name: &str, contents: &[u8]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "dictos-kaikki-test-{}-{}",
            std::process::id(),
            name
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("input.jsonl");
        File::create(&path).unwrap().write_all(contents).unwrap();
        path
    }

    #[test]
    fn importa_so_traducoes_do_idioma_alvo() {
        let input = write_fixture("plain", FIXTURE.as_bytes());
        let (entries, stats) = import_kaikki_translations(&input, "pt").unwrap();

        assert_eq!(stats.lines_read, 5);
        assert_eq!(stats.lines_ignored, 2); // sem tradução pt + linha inválida
        assert_eq!(entries.len(), 2);

        let apple = entries.iter().find(|e| e.word == "apple").unwrap();
        assert!(apple.senses.is_empty());
        assert_eq!(
            apple.translations,
            vec![Translation {
                lang: "pt".into(),
                word: "maçã".into()
            }]
        );

        let run = entries.iter().find(|e| e.word == "run").unwrap();
        let words: Vec<&str> = run.translations.iter().map(|t| t.word.as_str()).collect();
        assert_eq!(words, ["correr", "executar"]);
    }

    #[test]
    fn aceita_gzip() {
        let dir = std::env::temp_dir().join(format!("dictos-kaikki-test-{}-gz", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("input.jsonl.gz");
        let mut encoder = flate2::write::GzEncoder::new(
            File::create(&path).unwrap(),
            flate2::Compression::default(),
        );
        encoder.write_all(FIXTURE.as_bytes()).unwrap();
        encoder.finish().unwrap();

        let (entries, _) = import_kaikki_translations(&path, "pt").unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn notices_cc_by_sa() {
        let dir = std::env::temp_dir().join(format!(
            "dictos-kaikki-test-{}-notices",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_kaikki_notices(&dir).unwrap();
        let text = std::fs::read_to_string(dir.join("NOTICES")).unwrap();
        assert!(text.contains("CC") && text.contains("by-sa") || text.contains("Attribution-ShareAlike"));
    }
}
