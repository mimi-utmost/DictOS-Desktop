use std::collections::btree_map::Entry as MapEntry;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use dictos_core::normalize::normalize_query;
use dictos_core::{DictError, DictionaryEntry};

use crate::{ENTRIES_FILE, INDEX_FILE};

/// Constrói um dicionário no formato v1 a partir de um iterável de entradas.
pub struct DictionaryBuilder;

impl DictionaryBuilder {
    /// Escreve `index.fst` e `entries.bin` em `out_dir` (criado se não
    /// existir). Entradas cujo headword normalizado colide são mescladas em
    /// uma única entrada, unindo acepções e traduções.
    pub fn build(
        entries: impl IntoIterator<Item = DictionaryEntry>,
        out_dir: &Path,
    ) -> Result<(), DictError> {
        let merged = merge_entries(entries);

        fs::create_dir_all(out_dir)?;

        let entries_path = out_dir.join(ENTRIES_FILE);
        let mut writer = BufWriter::new(File::create(&entries_path)?);

        let mut offsets: BTreeMap<String, u64> = BTreeMap::new();
        let mut cursor: u64 = 0;
        for entry in merged.values() {
            let payload = bincode::serialize(entry)
                .map_err(|e| DictError::Format(format!("falha ao serializar entrada: {e}")))?;
            let len = u32::try_from(payload.len()).map_err(|_| {
                DictError::Format(format!("entrada '{}' excede o tamanho máximo", entry.word))
            })?;
            writer.write_all(&len.to_le_bytes())?;
            writer.write_all(&payload)?;
            offsets.insert(normalize_query(&entry.word), cursor);
            cursor += 4 + payload.len() as u64;
        }
        writer.flush()?;

        let mut fst_buf = Vec::new();
        let mut map_builder = fst::MapBuilder::new(&mut fst_buf)
            .map_err(|e| DictError::Format(format!("falha ao criar índice FST: {e}")))?;
        map_builder
            .extend_iter(offsets)
            .map_err(|e| DictError::Format(format!("falha ao escrever índice FST: {e}")))?;
        map_builder
            .finish()
            .map_err(|e| DictError::Format(format!("falha ao finalizar índice FST: {e}")))?;
        fs::write(out_dir.join(INDEX_FILE), fst_buf)?;

        Ok(())
    }
}

/// Agrupa por headword normalizado, mesclando duplicatas. A primeira ocorrência
/// fornece `word`/`lang`; acepções e traduções são unidas sem duplicatas.
fn merge_entries(
    entries: impl IntoIterator<Item = DictionaryEntry>,
) -> BTreeMap<String, DictionaryEntry> {
    let mut merged: BTreeMap<String, DictionaryEntry> = BTreeMap::new();
    for mut entry in entries {
        let key = normalize_query(&entry.word);
        match merged.entry(key) {
            MapEntry::Occupied(occupied) => {
                let existing = occupied.into_mut();
                for sense in entry.senses.drain(..) {
                    if !existing.senses.contains(&sense) {
                        existing.senses.push(sense);
                    }
                }
                for translation in entry.translations.drain(..) {
                    if !existing.translations.contains(&translation) {
                        existing.translations.push(translation);
                    }
                }
            }
            MapEntry::Vacant(vacant) => {
                vacant.insert(entry);
            }
        }
    }
    merged
}
