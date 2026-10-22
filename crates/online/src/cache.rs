//! Cache local de respostas online, no mesmo `dictos.db` das preferências.
//!
//! A palavra consultada é dado sensível: a chave é o SHA-256 da palavra
//! normalizada e o corpo da resposta é gravado criptografado via
//! [`dictos_history::Protector`]. Falha de proteção aborta a gravação.

use std::path::Path;

use dictos_history::{ProtectError, Protector};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("falha no banco: {0}")]
    Db(#[from] rusqlite::Error),
    #[error(transparent)]
    Protect(#[from] ProtectError),
    #[error("entrada de cache corrompida: {0}")]
    Corrupt(String),
}

/// Cache de respostas de provedores online.
pub struct OnlineCache<'p, P: Protector> {
    conn: Connection,
    protector: &'p P,
}

impl<'p, P: Protector> OnlineCache<'p, P> {
    pub fn open(db_path: impl AsRef<Path>, protector: &'p P) -> Result<Self, CacheError> {
        if let Some(parent) = db_path.as_ref().parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS online_cache(
                word_hash BLOB NOT NULL,
                provider TEXT NOT NULL,
                body_cipher BLOB NOT NULL,
                fetched_at INTEGER NOT NULL,
                PRIMARY KEY(word_hash, provider)
            );",
        )?;
        Ok(Self { conn, protector })
    }

    fn key(word: &str) -> Vec<u8> {
        Sha256::digest(word.trim().to_lowercase().as_bytes()).to_vec()
    }

    /// Corpo da resposta cacheada, se existir.
    pub fn get(&self, word: &str, provider: &str) -> Result<Option<String>, CacheError> {
        let row = self.conn.query_row(
            "SELECT body_cipher FROM online_cache WHERE word_hash = ?1 AND provider = ?2",
            rusqlite::params![Self::key(word), provider],
            |row| row.get::<_, Vec<u8>>(0),
        );
        let cipher = match row {
            Ok(cipher) => cipher,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(err) => return Err(err.into()),
        };
        let plain = self.protector.unprotect(&cipher)?;
        String::from_utf8(plain)
            .map(Some)
            .map_err(|e| CacheError::Corrupt(format!("UTF-8 inválido: {e}")))
    }

    /// Grava (ou substitui) a resposta de um provedor. Falha de proteção
    /// aborta a gravação — nada sensível em texto puro.
    pub fn put(&self, word: &str, provider: &str, body: &str) -> Result<(), CacheError> {
        let cipher = self.protector.protect(body.as_bytes())?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        self.conn.execute(
            "INSERT INTO online_cache(word_hash, provider, body_cipher, fetched_at)
             VALUES(?1, ?2, ?3, ?4)
             ON CONFLICT(word_hash, provider)
             DO UPDATE SET body_cipher = excluded.body_cipher, fetched_at = excluded.fetched_at",
            rusqlite::params![Self::key(word), provider, cipher, now],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct XorProtector;

    impl Protector for XorProtector {
        fn protect(&self, plaintext: &[u8]) -> Result<Vec<u8>, ProtectError> {
            Ok(plaintext.iter().map(|b| b ^ 0x5A).collect())
        }
        fn unprotect(&self, cipher: &[u8]) -> Result<Vec<u8>, ProtectError> {
            self.protect(cipher)
        }
    }

    fn temp_db(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "dictos-online-test-{}-{}",
            std::process::id(),
            name
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("dictos.db")
    }

    #[test]
    fn put_get_roundtrip() {
        let db = temp_db("roundtrip");
        let cache = OnlineCache::open(&db, &XorProtector).unwrap();
        assert_eq!(cache.get("apple", "dictionaryapi.dev").unwrap(), None);
        cache.put("apple", "dictionaryapi.dev", "[{\"word\":\"apple\"}]").unwrap();
        assert_eq!(
            cache.get("apple", "dictionaryapi.dev").unwrap().as_deref(),
            Some("[{\"word\":\"apple\"}]")
        );
        // Outro provedor não vê o cache deste.
        assert_eq!(cache.get("apple", "outro").unwrap(), None);
    }

    #[test]
    fn palavra_e_corpo_nao_ficam_em_texto_puro() {
        let db = temp_db("privacy");
        let cache = OnlineCache::open(&db, &XorProtector).unwrap();
        cache.put("serendipity", "dictionaryapi.dev", "definition body").unwrap();
        drop(cache);
        let bytes = std::fs::read(&db).unwrap();
        let haystack = String::from_utf8_lossy(&bytes);
        assert!(!haystack.contains("serendipity"));
        assert!(!haystack.contains("definition body"));
    }
}
