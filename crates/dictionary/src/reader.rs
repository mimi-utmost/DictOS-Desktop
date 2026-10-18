use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use dictos_core::normalize::{normalize_query, strip_diacritics};
use dictos_core::{DictError, DictionaryEntry, LookupResult};
use fst::{IntoStreamer, Streamer};

use crate::{ENTRIES_FILE, INDEX_FILE};

/// Leitor de um dicionário no formato v1. O índice FST é carregado inteiro em
/// memória na abertura; os payloads são lidos sob demanda por seek/offset.
pub struct Dictionary {
    index: fst::Map<Vec<u8>>,
    /// Índice secundário: headword sem diacríticos → offsets no `entries.bin`.
    ascii_index: HashMap<String, Vec<u64>>,
    entries: File,
}

impl Dictionary {
    /// Abre um dicionário a partir do diretório que contém `index.fst` e
    /// `entries.bin`.
    pub fn open(dir: &Path) -> Result<Self, DictError> {
        let index = fst::Map::new(fs::read(dir.join(INDEX_FILE))?)
            .map_err(|e| DictError::Format(format!("índice FST inválido: {e}")))?;
        let entries = File::open(dir.join(ENTRIES_FILE))?;

        let mut ascii_index: HashMap<String, Vec<u64>> = HashMap::new();
        let mut stream = index.stream();
        while let Some((key, offset)) = stream.next() {
            let key = String::from_utf8_lossy(key).into_owned();
            ascii_index
                .entry(strip_diacritics(&key))
                .or_default()
                .push(offset);
        }

        Ok(Self {
            index,
            ascii_index,
            entries,
        })
    }

    /// Consulta exata pelo headword normalizado. Se não houver casamento,
    /// tenta o fallback sem diacríticos. Erro `NotFound` caso contrário.
    pub fn lookup(&self, word: &str) -> Result<LookupResult, DictError> {
        let key = normalize_query(word);
        if let Some(&offset) = self.index.get(&key).as_ref() {
            return Ok(LookupResult {
                entry: self.read_entry(offset)?,
                diacritic_fallback: false,
            });
        }

        let ascii_key = strip_diacritics(&key);
        if let Some(offsets) = self.ascii_index.get(&ascii_key) {
            if let Some(&offset) = offsets.first() {
                return Ok(LookupResult {
                    entry: self.read_entry(offset)?,
                    diacritic_fallback: true,
                });
            }
        }

        Err(DictError::NotFound(key))
    }

    /// Autocomplete: headwords que começam com o prefixo (normalizado),
    /// limitado a `limit` itens.
    pub fn lookup_prefix(&self, prefix: &str, limit: usize) -> Vec<String> {
        let prefix = normalize_query(prefix);
        if prefix.is_empty() || limit == 0 {
            return Vec::new();
        }
        self.index
            .range()
            .ge(&prefix)
            .into_stream()
            .into_str_keys()
            .map(|keys| {
                keys.into_iter()
                    .take_while(|key| key.starts_with(&prefix))
                    .take(limit)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn read_entry(&self, offset: u64) -> Result<DictionaryEntry, DictError> {
        let mut file = &self.entries;
        file.seek(SeekFrom::Start(offset))?;

        let mut len_buf = [0u8; 4];
        file.read_exact(&mut len_buf)?;
        let len = u32::from_le_bytes(len_buf) as usize;

        let mut payload = vec![0u8; len];
        file.read_exact(&mut payload)?;

        bincode::deserialize(&payload)
            .map_err(|e| DictError::Format(format!("payload corrompido no offset {offset}: {e}")))
    }
}
