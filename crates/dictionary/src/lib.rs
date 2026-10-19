//! Formato de dicionário v1 do DictOS.
//!
//! Um dicionário é um diretório com dois arquivos:
//!
//! - `index.fst`: mapa FST de headword normalizado (minúsculas, sem espaços
//!   extras) para offset `u64` dentro de `entries.bin`.
//! - `entries.bin`: sequência de registros `[u32 LE tamanho][payload bincode
//!   do DictionaryEntry]`.
//!
//! A busca sem diacríticos funciona via um segundo índice em memória,
//! construído na abertura: headword sem diacríticos → offset.

mod builder;
mod import;
mod reader;
pub mod wordnet;

pub use builder::DictionaryBuilder;
pub use import::import_jsonl;
pub use reader::Dictionary;

/// Nome do arquivo de índice FST dentro do diretório do dicionário.
pub const INDEX_FILE: &str = "index.fst";
/// Nome do arquivo de payloads dentro do diretório do dicionário.
pub const ENTRIES_FILE: &str = "entries.bin";
