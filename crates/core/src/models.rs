use serde::{Deserialize, Serialize};

/// Uma entrada de dicionário: um headword com seus sentidos e traduções.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictionaryEntry {
    /// Forma canônica da palavra (ex.: "run", "maçã").
    pub word: String,
    /// Idioma da palavra, código ISO 639-1 (ex.: "en", "pt").
    pub lang: String,
    /// Acepções da palavra, agrupadas por classe gramatical.
    pub senses: Vec<Sense>,
    /// Traduções para outros idiomas.
    pub translations: Vec<Translation>,
}

/// Uma acepção de uma palavra dentro de uma classe gramatical.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sense {
    /// Classe gramatical (ex.: "noun", "verb", "adjective").
    pub pos: String,
    /// Definição da acepção.
    pub definition: String,
    /// Frases de exemplo que usam a palavra nesta acepção.
    pub examples: Vec<String>,
}

/// Tradução da palavra para outro idioma.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Translation {
    /// Idioma de destino, código ISO 639-1 (ex.: "pt").
    pub lang: String,
    /// Palavra traduzida.
    pub word: String,
}

/// Resultado de uma consulta, indicando se o headword casou exatamente com a
/// consulta ou se foi encontrado via fallback sem diacríticos.
#[derive(Debug, Clone, PartialEq)]
pub struct LookupResult {
    pub entry: DictionaryEntry,
    /// `true` quando a entrada foi encontrada pelo fallback sem diacríticos
    /// (a consulta normalizada não casou com nenhum headword exato).
    pub diacritic_fallback: bool,
}
