//! Provedores online de dicionário (opcionais) e cache local.
//!
//! Privacidade: nenhum fetch acontece sem ação explícita do usuário (botão
//! "Consultar online" no popup ou `--online` no CLI). O cache grava a
//! resposta criptografada via [`dictos_history::Protector`], indexada por
//! hash da palavra — nada sensível em texto puro em repouso.

pub mod cache;
pub mod dictionaryapi;

pub use cache::OnlineCache;
pub use dictionaryapi::DictionaryApiDev;

use dictos_core::DictionaryEntry;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OnlineError {
    #[error("sem conexão ou provedor indisponível: {0}")]
    Network(String),
    #[error("o provedor respondeu com HTTP {0}")]
    Http(u16),
    #[error("palavra não encontrada no provedor online")]
    NotFound,
    #[error("resposta do provedor não pôde ser interpretada: {0}")]
    Parse(String),
}

/// Resultado de uma consulta online, pronto para exibição/cache.
#[derive(Debug, Clone)]
pub struct OnlineResult {
    pub entry: DictionaryEntry,
    pub provider: &'static str,
    /// Corpo bruto da resposta (é o que o cache armazena).
    pub raw_body: String,
}

/// Provedor remoto de definições. Implementações devem ser síncronas
/// (o daemon as chama da thread worker).
pub trait OnlineProvider {
    fn name(&self) -> &'static str;
    fn fetch(&self, word: &str) -> Result<OnlineResult, OnlineError>;
}
