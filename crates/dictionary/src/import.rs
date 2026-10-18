use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use dictos_core::{DictError, DictionaryEntry};

/// Itera sobre um arquivo JSONL de intercâmbio, uma entrada por linha, cada
/// linha um `DictionaryEntry` serializado em JSON. Linhas em branco são
/// ignoradas; uma linha malformada produz `DictError::Format` com o número da
/// linha (base 1).
pub fn import_jsonl(
    path: &Path,
) -> impl Iterator<Item = Result<DictionaryEntry, DictError>> {
    let file = File::open(path).map_err(DictError::Io);
    file.into_iter().flat_map(move |f| {
        BufReader::new(f)
            .lines()
            .enumerate()
            .filter(|(_, line)| line.as_ref().map(|l| !l.trim().is_empty()).unwrap_or(true))
            .map(|(idx, line)| {
                let line = line.map_err(DictError::Io)?;
                serde_json::from_str::<DictionaryEntry>(&line).map_err(|e| {
                    DictError::Format(format!("linha {}: JSON inválido: {e}", idx + 1))
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
    })
}
