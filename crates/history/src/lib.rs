//! Histórico opcional de consultas (opt-in, somente local).
//!
//! As palavras consultadas são seleções do usuário — dado sensível. Por
//! isso a palavra é **sempre** criptografada antes de tocar o disco, via a
//! trait [`Protector`] (no Windows, `DpapiProtector` do crate
//! `platform-windows`). Se a proteção falhar, a entrada não é gravada —
//! nunca se grava a palavra em texto puro.

use std::path::Path;

use rusqlite::Connection;
use thiserror::Error;

/// Falha ao proteger/desproteger um dado sensível.
#[derive(Debug, Error)]
#[error("falha na proteção de dado sensível: {0}")]
pub struct ProtectError(pub String);

/// Cifra de dados sensíveis em repouso (DPAPI no Windows, keyring no Linux).
pub trait Protector {
    fn protect(&self, plaintext: &[u8]) -> Result<Vec<u8>, ProtectError>;
    fn unprotect(&self, cipher: &[u8]) -> Result<Vec<u8>, ProtectError>;
}

/// Uma entrada do histórico, já descriptografada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub word: String,
    pub found: bool,
    /// Segundos desde a época Unix (UTC).
    pub looked_up_at: i64,
}

#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("falha no banco: {0}")]
    Db(#[from] rusqlite::Error),
    #[error(transparent)]
    Protect(#[from] ProtectError),
    #[error("entrada corrompida: {0}")]
    Corrupt(String),
}

/// Histórico de consultas num banco SQLite compartilhado com `settings`.
pub struct History<'p, P: Protector> {
    conn: Connection,
    protector: &'p P,
}

impl<'p, P: Protector> History<'p, P> {
    /// Abre o histórico no mesmo banco das preferências.
    pub fn open(db_path: impl AsRef<Path>, protector: &'p P) -> Result<Self, HistoryError> {
        if let Some(parent) = db_path.as_ref().parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history(
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                word_cipher BLOB NOT NULL,
                found INTEGER NOT NULL,
                looked_up_at INTEGER NOT NULL
            );",
        )?;
        Ok(Self { conn, protector })
    }

    /// Registra uma consulta. Falha de proteção aborta a gravação (a palavra
    /// nunca é persistida em texto puro).
    pub fn record(&self, word: &str, found: bool) -> Result<(), HistoryError> {
        let cipher = self.protector.protect(word.as_bytes())?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        self.conn.execute(
            "INSERT INTO history(word_cipher, found, looked_up_at) VALUES(?1, ?2, ?3)",
            rusqlite::params![cipher, found as i32, now],
        )?;
        Ok(())
    }

    /// Lista as entradas mais recentes primeiro.
    pub fn list(&self, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        let mut stmt = self.conn.prepare(
            "SELECT word_cipher, found, looked_up_at FROM history ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit as i64], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?;
        let mut entries = Vec::new();
        for row in rows {
            let (cipher, found, looked_up_at) = row?;
            let plain = self.protector.unprotect(&cipher)?;
            let word = String::from_utf8(plain)
                .map_err(|e| HistoryError::Corrupt(format!("UTF-8 inválido: {e}")))?;
            entries.push(HistoryEntry {
                word,
                found: found != 0,
                looked_up_at,
            });
        }
        Ok(entries)
    }

    /// Apaga todo o histórico.
    pub fn clear(&self) -> Result<(), HistoryError> {
        self.conn.execute("DELETE FROM history", [])?;
        Ok(())
    }
}

/// Formata segundos Unix (UTC) como `YYYY-MM-DD HH:MM:SS` sem dependências
/// (algoritmo civil_from_days de Howard Hinnant).
pub fn format_utc(epoch_secs: i64) -> String {
    let days = epoch_secs.div_euclid(86_400);
    let secs = epoch_secs.rem_euclid(86_400);
    let (hour, minute, second) = (secs / 3600, (secs % 3600) / 60, secs % 60);

    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };

    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Protetor de teste: XOR com constante. NÃO é criptografia — serve para
    /// provar o encanamento e que a palavra não vai em texto puro ao disco.
    struct XorProtector;

    impl Protector for XorProtector {
        fn protect(&self, plaintext: &[u8]) -> Result<Vec<u8>, ProtectError> {
            Ok(plaintext.iter().map(|b| b ^ 0x5A).collect())
        }
        fn unprotect(&self, cipher: &[u8]) -> Result<Vec<u8>, ProtectError> {
            self.protect(cipher)
        }
    }

    /// Protetor que sempre falha: a gravação deve ser abortada.
    struct FailingProtector;

    impl Protector for FailingProtector {
        fn protect(&self, _: &[u8]) -> Result<Vec<u8>, ProtectError> {
            Err(ProtectError("simulado".into()))
        }
        fn unprotect(&self, _: &[u8]) -> Result<Vec<u8>, ProtectError> {
            Err(ProtectError("simulado".into()))
        }
    }

    fn temp_db(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "dictos-history-test-{}-{}",
            std::process::id(),
            name
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("dictos.db")
    }

    #[test]
    fn record_e_list_na_ordem_recente_primeiro() {
        let db = temp_db("list");
        let history = History::open(&db, &XorProtector).unwrap();
        history.record("apple", true).unwrap();
        history.record("xyzzy", false).unwrap();
        let entries = history.list(10).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].word, "xyzzy");
        assert!(!entries[0].found);
        assert_eq!(entries[1].word, "apple");
        assert!(entries[1].found);
    }

    #[test]
    fn list_respeita_limite() {
        let db = temp_db("limit");
        let history = History::open(&db, &XorProtector).unwrap();
        for word in ["a", "b", "c"] {
            history.record(word, true).unwrap();
        }
        assert_eq!(history.list(2).unwrap().len(), 2);
    }

    #[test]
    fn clear_apaga_tudo() {
        let db = temp_db("clear");
        let history = History::open(&db, &XorProtector).unwrap();
        history.record("apple", true).unwrap();
        history.clear().unwrap();
        assert!(history.list(10).unwrap().is_empty());
    }

    #[test]
    fn palavra_nao_vai_em_texto_puro_ao_disco() {
        let db = temp_db("privacy");
        let history = History::open(&db, &XorProtector).unwrap();
        history.record("serendipity", true).unwrap();
        drop(history);
        let bytes = std::fs::read(&db).unwrap();
        let haystack = String::from_utf8_lossy(&bytes);
        assert!(!haystack.contains("serendipity"));
    }

    #[test]
    fn falha_de_protecao_aborta_gravacao() {
        let db = temp_db("fail");
        let history = History::open(&db, &FailingProtector).unwrap();
        assert!(history.record("apple", true).is_err());
        let count: i64 = history
            .conn
            .query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn formata_epoch_utc() {
        assert_eq!(format_utc(0), "1970-01-01 00:00:00");
        assert_eq!(format_utc(1_700_000_000), "2023-11-14 22:13:20");
    }
}
