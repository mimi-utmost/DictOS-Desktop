//! Provedor dictionaryapi.dev — gratuito, sem chave, en–en.
//!
//! Resposta: array de objetos `{ word, meanings: [{ partOfSpeech,
//! definitions: [{ definition, example? }] }] }`.

use dictos_core::{DictionaryEntry, Sense};
use serde::Deserialize;

use crate::{OnlineError, OnlineProvider, OnlineResult};

/// Provedor https://dictionaryapi.dev (en–en, sem chave de API).
pub struct DictionaryApiDev;

const BASE_URL: &str = "https://api.dictionaryapi.dev/api/v2/entries/en";

#[derive(Deserialize)]
struct ApiEntry {
    word: String,
    #[serde(default)]
    meanings: Vec<ApiMeaning>,
}

#[derive(Deserialize)]
struct ApiMeaning {
    #[serde(rename = "partOfSpeech")]
    part_of_speech: String,
    #[serde(default)]
    definitions: Vec<ApiDefinition>,
}

#[derive(Deserialize)]
struct ApiDefinition {
    definition: String,
    example: Option<String>,
}

impl OnlineProvider for DictionaryApiDev {
    fn name(&self) -> &'static str {
        "dictionaryapi.dev"
    }

    fn fetch(&self, word: &str) -> Result<OnlineResult, OnlineError> {
        let url = format!("{BASE_URL}/{}", url_encode(word));
        let response = ureq::get(&url)
            .call()
            .map_err(|err| match err {
                ureq::Error::StatusCode(404) => OnlineError::NotFound,
                ureq::Error::StatusCode(code) => OnlineError::Http(code),
                other => OnlineError::Network(other.to_string()),
            })?;
        let body = response
            .into_body()
            .read_to_string()
            .map_err(|e| OnlineError::Network(e.to_string()))?;
        let entry = parse_entries(&body)?;
        Ok(OnlineResult {
            entry,
            provider: self.name(),
            raw_body: body,
        })
    }
}

/// Parse puro da resposta JSON — testável sem rede.
pub fn parse_entries(body: &str) -> Result<DictionaryEntry, OnlineError> {
    let api: Vec<ApiEntry> =
        serde_json::from_str(body).map_err(|e| OnlineError::Parse(e.to_string()))?;
    let first = api.into_iter().next().ok_or(OnlineError::NotFound)?;

    let mut senses = Vec::new();
    for meaning in first.meanings {
        for definition in meaning.definitions {
            senses.push(Sense {
                pos: meaning.part_of_speech.clone(),
                definition: definition.definition,
                examples: definition.example.into_iter().collect(),
            });
        }
    }
    if senses.is_empty() {
        return Err(OnlineError::NotFound);
    }
    Ok(DictionaryEntry {
        word: first.word,
        lang: "en".into(),
        senses,
        translations: Vec::new(),
    })
}

/// Mínimo de percent-encoding para path segment (a consulta já passou por
/// `extract_query`: no máximo 5 palavras, 100 chars).
fn url_encode(word: &str) -> String {
    let mut out = String::new();
    for byte in word.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Estrutura real da resposta do dictionaryapi.dev (encurtada).
    const FIXTURE: &str = r#"[{
        "word": "serendipity",
        "phonetics": [],
        "meanings": [{
            "partOfSpeech": "noun",
            "definitions": [
                {"definition": "The occurrence and development of events by chance in a happy or beneficial way.", "example": "a fortunate stroke of serendipity"},
                {"definition": "The faculty of making happy and unexpected discoveries by accident."}
            ]
        }]
    }]"#;

    #[test]
    fn parse_resposta_real() {
        let entry = parse_entries(FIXTURE).unwrap();
        assert_eq!(entry.word, "serendipity");
        assert_eq!(entry.senses.len(), 2);
        assert_eq!(entry.senses[0].pos, "noun");
        assert!(entry.senses[0].definition.contains("by chance"));
        assert_eq!(entry.senses[0].examples, vec!["a fortunate stroke of serendipity"]);
        assert!(entry.senses[1].examples.is_empty());
    }

    #[test]
    fn parse_sem_acepcoes_e_notfound() {
        assert!(matches!(parse_entries("[]"), Err(OnlineError::NotFound)));
        assert!(matches!(
            parse_entries(r#"[{"word":"x","meanings":[]}]"#),
            Err(OnlineError::NotFound)
        ));
    }

    #[test]
    fn url_encode_seguro() {
        assert_eq!(url_encode("ice cream"), "ice%20cream");
        assert_eq!(url_encode("apple"), "apple");
    }
}
