//! Integração do DictOS com o Windows.
//!
//! Compila apenas no Windows; em outros sistemas o crate existe mas não
//! expõe funcionalidade (o núcleo permanece portável).

#[cfg(windows)]
pub mod clipboard_fallback;
#[cfg(windows)]
pub mod hotkey;
#[cfg(windows)]
pub mod uia;
