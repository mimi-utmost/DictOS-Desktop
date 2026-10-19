use std::io;

/// Erros do núcleo do DictOS.
#[derive(Debug, thiserror::Error)]
pub enum DictError {
    /// Palavra não encontrada no dicionário.
    #[error("palavra não encontrada: {0}")]
    NotFound(String),

    /// Falha de I/O ao ler ou escrever arquivos do dicionário.
    #[error("erro de I/O: {0}")]
    Io(#[from] io::Error),

    /// Dicionário ou entrada em formato inválido.
    #[error("formato inválido: {0}")]
    Format(String),
}
