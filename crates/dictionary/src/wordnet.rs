//! Importador do formato de banco de dados do Princeton WordNet 3.1
//! (arquivos `index.{noun,verb,adj,adv}` e `data.{noun,verb,adj,adv}`).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use dictos_core::{DictError, DictionaryEntry, Sense};

/// (arquivo de índice, arquivo de dados) por classe gramatical do WordNet.
const POS_FILES: [(&str, &str); 4] = [
    ("index.noun", "data.noun"),
    ("index.verb", "data.verb"),
    ("index.adj", "data.adj"),
    ("index.adv", "data.adv"),
];

/// Uma acepção extraída de um arquivo `data.*`, indexada pelo offset do synset.
struct Gloss {
    pos: String,
    definition: String,
    examples: Vec<String>,
}

/// Importa um diretório `dict/` do WordNet 3.1 para entradas do DictOS.
///
/// Cada lema vira uma entrada por classe gramatical; o `DictionaryBuilder`
/// mescla lemas que aparecem em mais de uma classe (ex.: "run" como verbo e
/// substantivo). A ordem das acepções segue a ordem dos offsets no índice,
/// que no WordNet reflete a frequência de uso.
pub fn import_wordnet(dict_dir: &Path) -> Result<Vec<DictionaryEntry>, DictError> {
    let mut entries = Vec::new();

    for (index_name, data_name) in POS_FILES {
        let glosses = parse_data_file(&dict_dir.join(data_name))?;
        let index_content = read_wn_file(&dict_dir.join(index_name))?;

        for (line_no, line) in index_content.lines().enumerate() {
            if line.starts_with("  ") || line.trim().is_empty() {
                continue;
            }
            let (lemma, offsets) = parse_index_line(line).map_err(|e| {
                DictError::Format(format!("{index_name}:{}: {e}", line_no + 1))
            })?;

            let senses: Vec<Sense> = offsets
                .iter()
                .filter_map(|offset| glosses.get(offset))
                .map(|g| Sense {
                    pos: g.pos.clone(),
                    definition: g.definition.clone(),
                    examples: g.examples.clone(),
                })
                .collect();

            if senses.is_empty() {
                continue;
            }

            entries.push(DictionaryEntry {
                word: lemma.replace('_', " "),
                lang: "en".to_string(),
                senses,
                translations: Vec::new(),
            });
        }
    }

    Ok(entries)
}

/// Lê um arquivo do WordNet. Os arquivos oficiais são ASCII; caracteres fora
/// de ASCII aparecem na codificação "Windows-1252" em alguns glosses, então
/// fazemos a conversão byte a byte para preservá-los.
fn read_wn_file(path: &Path) -> Result<String, DictError> {
    let bytes = fs::read(path)?;
    Ok(bytes.iter().map(|&b| b as char).collect())
}

/// Analisa uma linha de `index.*`:
/// `lemma pos synset_cnt p_cnt [ptr...] sense_cnt tagsense_cnt offset...`
fn parse_index_line(line: &str) -> Result<(String, Vec<u64>), String> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.len() < 6 {
        return Err(format!("linha curta demais: '{line}'"));
    }
    let lemma = tokens[0].to_string();
    let synset_cnt: usize = tokens[2]
        .parse()
        .map_err(|_| format!("synset_cnt inválido: '{}'", tokens[2]))?;
    let p_cnt: usize = tokens[3]
        .parse()
        .map_err(|_| format!("p_cnt inválido: '{}'", tokens[3]))?;

    let offsets_start = 4 + p_cnt + 2;
    let offsets_end = offsets_start + synset_cnt;
    if tokens.len() < offsets_end {
        return Err(format!("esperados {offsets_end} campos: '{line}'"));
    }

    let mut offsets = Vec::with_capacity(synset_cnt);
    for token in &tokens[offsets_start..offsets_end] {
        offsets.push(
            token
                .parse()
                .map_err(|_| format!("offset inválido: '{token}'"))?,
        );
    }
    Ok((lemma, offsets))
}

/// Analisa um arquivo `data.*` inteiro para um mapa offset → acepção.
fn parse_data_file(path: &Path) -> Result<HashMap<u64, Gloss>, DictError> {
    let content = read_wn_file(path)?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut glosses = HashMap::new();

    for (line_no, line) in content.lines().enumerate() {
        if line.starts_with("  ") || line.trim().is_empty() {
            continue;
        }
        let (offset, gloss) = parse_data_line(line)
            .map_err(|e| DictError::Format(format!("{name}:{}: {e}", line_no + 1)))?;
        glosses.insert(offset, gloss);
    }
    Ok(glosses)
}

/// Analisa uma linha de `data.*`:
/// `offset lex_filenum ss_type w_cnt [palavra lex_id]... p_cnt [ptr...] | gloss`
///
/// Só os três primeiros campos e o gloss nos interessam; `w_cnt` é
/// hexadecimal, mas não precisamos percorrer a lista de palavras.
fn parse_data_line(line: &str) -> Result<(u64, Gloss), String> {
    let (left, gloss_text) = line
        .split_once('|')
        .ok_or_else(|| format!("falta o separador '|': '{line}'"))?;

    let mut tokens = left.split_whitespace();
    let offset: u64 = tokens
        .next()
        .ok_or("linha vazia")?
        .parse()
        .map_err(|_| "offset inválido".to_string())?;
    tokens.next(); // lex_filenum
    let ss_type = tokens.next().ok_or("falta ss_type")?;

    let pos = match ss_type {
        "n" => "noun",
        "v" => "verb",
        "a" | "s" => "adjective",
        "r" => "adverb",
        other => return Err(format!("ss_type desconhecido: '{other}'")),
    }
    .to_string();

    let (definition, examples) = split_gloss(gloss_text);
    Ok((
        offset,
        Gloss {
            pos,
            definition,
            examples,
        },
    ))
}

/// Separa o gloss do WordNet em definição e exemplos. O formato é
/// `definição; "exemplo"; "exemplo"`; segmentos sem aspas após o primeiro
/// são tratados como continuação da definição.
fn split_gloss(gloss: &str) -> (String, Vec<String>) {
    let mut definition_parts = Vec::new();
    let mut examples = Vec::new();

    for segment in gloss.split(';') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        if segment.contains('"') {
            let mut rest = segment;
            while let Some(start) = rest.find('"') {
                rest = &rest[start + 1..];
                match rest.find('"') {
                    Some(end) => {
                        let example = rest[..end].trim();
                        if !example.is_empty() {
                            examples.push(example.to_string());
                        }
                        rest = &rest[end + 1..];
                    }
                    None => break,
                }
            }
        } else {
            definition_parts.push(segment);
        }
    }

    (definition_parts.join("; "), examples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_gloss_definicao_simples() {
        let (def, ex) = split_gloss(" fruit with red or yellow skin  ");
        assert_eq!(def, "fruit with red or yellow skin");
        assert!(ex.is_empty());
    }

    #[test]
    fn split_gloss_com_exemplos() {
        let (def, ex) = split_gloss(
            r#" a score in baseball made by a runner touching all bases safely; "the Yankees scored 3 runs in the bottom of the 9th"; "their first tally came in the 3rd inning" "#,
        );
        assert_eq!(def, "a score in baseball made by a runner touching all bases safely");
        assert_eq!(ex.len(), 2);
        assert_eq!(ex[0], "the Yankees scored 3 runs in the bottom of the 9th");
    }

    #[test]
    fn split_gloss_definicao_em_dois_segmentos() {
        let (def, ex) = split_gloss(" the act of testing something; a trying of something ");
        assert_eq!(def, "the act of testing something; a trying of something");
        assert!(ex.is_empty());
    }

    #[test]
    fn parse_index_line_com_offsets() {
        let (lemma, offsets) = parse_index_line("apple n 2 5 @ ~ #m #p %p 2 1 07755101 12654755").unwrap();
        assert_eq!(lemma, "apple");
        assert_eq!(offsets, vec![7755101, 12654755]);
    }

    #[test]
    fn parse_data_line_noun() {
        let (offset, gloss) = parse_data_line(
            "07755101 13 n 01 apple 0 006 @ 07721676 n 0000 | fruit with red skin  ",
        )
        .unwrap();
        assert_eq!(offset, 7755101);
        assert_eq!(gloss.pos, "noun");
        assert_eq!(gloss.definition, "fruit with red skin");
    }

    #[test]
    fn parse_data_line_adjective_satellite() {
        let (_, gloss) = parse_data_line("00002098 00 s 02 able 0 capable 0 | having ability  ").unwrap();
        assert_eq!(gloss.pos, "adjective");
    }
}
