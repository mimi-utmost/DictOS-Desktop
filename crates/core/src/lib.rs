//! Núcleo do DictOS: modelos de dicionário, normalização de consultas e erros.
//!
//! Este crate não faz I/O de plataforma nem depende de GUI, para permanecer
//! testável em qualquer ambiente.

pub mod normalize;

mod error;
mod models;

pub use error::DictError;
pub use models::{DictionaryEntry, LookupResult, Sense, Translation};
