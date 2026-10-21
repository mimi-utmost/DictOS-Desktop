//! Preferências do DictOS, persistidas em SQLite.
//!
//! Banco único em `%APPDATA%\DictOS\dictos.db` (override via variável de
//! ambiente `DICTOS_DATA_DIR`, usada nos testes). O schema é versionado por
//! `PRAGMA user_version`: a v1 cria a tabela `settings`; crates que
//! compartilham o arquivo (ex.: `history`) criam suas próprias tabelas de
//! forma idempotente.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

pub const KEY_HOTKEY: &str = "hotkey";
pub const KEY_HISTORY_ENABLED: &str = "history.enabled";
pub const KEY_FONT_SCALE: &str = "popup.font_scale";

pub const DEFAULT_HOTKEY: &str = "Ctrl+Shift+D";

/// Defaults conhecidos, na ordem de exibição.
pub const DEFAULTS: &[(&str, &str)] = &[
    (KEY_HOTKEY, DEFAULT_HOTKEY),
    (KEY_HISTORY_ENABLED, "false"),
    (KEY_FONT_SCALE, "1.0"),
];

/// Diretório de dados do aplicativo.
pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("DICTOS_DATA_DIR") {
        return PathBuf::from(dir);
    }
    let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
    PathBuf::from(base).join("DictOS")
}

/// Caminho do banco SQLite único.
pub fn db_path() -> PathBuf {
    data_dir().join("dictos.db")
}

/// Acesso às preferências.
pub struct Settings {
    conn: Connection,
}

impl Settings {
    /// Abre (criando, se preciso) o banco padrão do usuário.
    pub fn open() -> rusqlite::Result<Self> {
        Self::open_at(db_path())
    }

    /// Abre (criando, se preciso) um banco num caminho específico.
    pub fn open_at(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        if let Some(parent) = path.as_ref().parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path)?;
        let settings = Self { conn };
        settings.migrate()?;
        Ok(settings)
    }

    fn migrate(&self) -> rusqlite::Result<()> {
        let version: u32 = self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version < 1 {
            self.conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS settings(
                    key   TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                PRAGMA user_version = 1;",
            )?;
        }
        Ok(())
    }

    /// Valor de uma chave, se definido.
    pub fn get(&self, key: &str) -> rusqlite::Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                [key],
                |row| row.get(0),
            )
            .map(Some)
            .or_else(|err| match err {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })
    }

    /// Valor de uma chave, caindo no default conhecido (ou no informado).
    pub fn get_or(&self, key: &str, fallback: &str) -> String {
        self.get(key).ok().flatten().unwrap_or_else(|| fallback.into())
    }

    /// Valor efetivo de uma chave conhecida (default registrado em DEFAULTS).
    pub fn get_effective(&self, key: &str) -> String {
        let default = DEFAULTS
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
            .unwrap_or("");
        self.get_or(key, default)
    }

    /// Define uma chave.
    pub fn set(&self, key: &str, value: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO settings(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [key, value],
        )?;
        Ok(())
    }

    /// Lista (chave, valor) das preferências definidas explicitamente.
    pub fn list(&self) -> rusqlite::Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, value FROM settings ORDER BY key")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect()
    }

    /// Conexão subjacente (compartilhada com o crate `history`).
    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "dictos-settings-test-{}-{}",
            std::process::id(),
            name
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("dictos.db")
    }

    #[test]
    fn set_get_e_list() {
        let db = temp_db("setget");
        let settings = Settings::open_at(&db).unwrap();
        assert_eq!(settings.get(KEY_HOTKEY).unwrap(), None);
        settings.set(KEY_HOTKEY, "Ctrl+Alt+D").unwrap();
        assert_eq!(
            settings.get(KEY_HOTKEY).unwrap().as_deref(),
            Some("Ctrl+Alt+D")
        );
        settings.set(KEY_HOTKEY, "Ctrl+F9").unwrap();
        let list = settings.list().unwrap();
        assert_eq!(list, vec![(KEY_HOTKEY.to_string(), "Ctrl+F9".to_string())]);
    }

    #[test]
    fn persiste_entre_aberturas() {
        let db = temp_db("persist");
        Settings::open_at(&db)
            .unwrap()
            .set(KEY_HISTORY_ENABLED, "true")
            .unwrap();
        let reopened = Settings::open_at(&db).unwrap();
        assert_eq!(reopened.get_or(KEY_HISTORY_ENABLED, "false"), "true");
    }

    #[test]
    fn defaults_conhecidos() {
        let db = temp_db("defaults");
        let settings = Settings::open_at(&db).unwrap();
        assert_eq!(settings.get_effective(KEY_HOTKEY), DEFAULT_HOTKEY);
        assert_eq!(settings.get_effective(KEY_HISTORY_ENABLED), "false");
        assert_eq!(settings.get_effective(KEY_FONT_SCALE), "1.0");
    }
}
