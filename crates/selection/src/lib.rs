//! Tipos e lógica de captura de seleção do DictOS.
//!
//! Este crate é independente de sistema operacional: define o contrato
//! ([`SelectionProvider`]) que as integrações de plataforma implementam e a
//! lógica pura de transformação da seleção bruta em consulta de dicionário
//! ([`extract_query`]).

use thiserror::Error;

/// Retângulo em coordenadas de tela (pixels físicos).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Texto capturado de outro aplicativo, com a posição da seleção quando
/// disponível (usada para posicionar o popup).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedSelection {
    pub text: String,
    pub anchor: Option<ScreenRect>,
}

/// Falhas possíveis na captura de seleção.
#[derive(Debug, Error)]
pub enum SelectionError {
    /// O aplicativo em foco não tem texto selecionado (ou não expõe seleção).
    #[error("nenhum texto selecionado")]
    NoSelection,
    /// A captura falhou por limitação do aplicativo ou do sistema.
    #[error("falha na captura: {0}")]
    CaptureFailed(String),
    /// O conteúdo da área de transferência não pôde ser restaurado após o
    /// fallback; a consulta é abortada para não perder dados do usuário.
    #[error("não foi possível restaurar a área de transferência")]
    ClipboardRestoreFailed,
}

/// Origem de captura de seleção de texto de outros aplicativos.
pub trait SelectionProvider {
    fn capture(&self) -> Result<CapturedSelection, SelectionError>;
}

/// Tamanho máximo de uma consulta, em caracteres.
const MAX_QUERY_CHARS: usize = 100;
/// Máximo de palavras de uma consulta (dicionário consulta palavra ou
/// expressão curta, não parágrafos).
const MAX_QUERY_WORDS: usize = 5;

/// Transforma o texto bruto de uma seleção em consulta de dicionário.
///
/// Colapsa quebras de linha e espaços repetidos, remove espaços das bordas e
/// rejeita seleções vazias ou longas demais para serem uma palavra/expressão.
pub fn extract_query(selection: &str) -> Option<String> {
    let normalized: String = selection.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty()
        || normalized.chars().count() > MAX_QUERY_CHARS
        || normalized.split_whitespace().count() > MAX_QUERY_WORDS
    {
        return None;
    }
    Some(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extrai_palavra_simples() {
        assert_eq!(extract_query("serendipity"), Some("serendipity".into()));
    }

    #[test]
    fn colapsa_espacos_e_quebras() {
        assert_eq!(
            extract_query("  ice\n\tcream  "),
            Some("ice cream".into())
        );
    }

    #[test]
    fn rejeita_vazio() {
        assert_eq!(extract_query(""), None);
        assert_eq!(extract_query("   \n\t  "), None);
    }

    #[test]
    fn rejeita_mais_de_cinco_palavras() {
        assert_eq!(extract_query("uma duas tres quatro cinco"), Some("uma duas tres quatro cinco".into()));
        assert_eq!(extract_query("uma duas tres quatro cinco seis"), None);
    }

    #[test]
    fn rejeita_texto_longo() {
        let longo = "a".repeat(101);
        assert_eq!(extract_query(&longo), None);
    }
}
