// SPDX-License-Identifier: MIT
//
// Copyright (c) 2025 Lorenzo Carbonell
//
// This file is part of the LiLa project,
// and is licensed under the MIT License. See the LICENSE file for details.
//
//! Historial de uso de las aplicaciones lanzadas, persistido en SQLite.
//!
//! Guarda, por cada ítem, cuántas veces se ha lanzado y cuándo fue la primera y
//! la última vez. Sobre esos datos se calcula una puntuación que combina
//! frecuencia y recencia, que el ranking usará más adelante.

use std::fmt;
use std::path::Path;
use std::sync::Mutex;

use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension};
use tracing::debug;

use crate::utils::{APP_APPLICATION, APP_ORGANIZATION, APP_QUALIFIER};

/// Vida media, en días, del decaimiento exponencial de la puntuación.
///
/// Cada `HALF_LIFE_DAYS` días sin usar un ítem, su puntuación se reduce a la
/// mitad respecto a la que tendría con el mismo número de lanzamientos recién
/// usados.
// TODO(core-lanzamiento): se consumirá en la Fase B2; retirar el allow entonces.
#[allow(dead_code)]
pub const HALF_LIFE_DAYS: f64 = 14.0;

/// Milisegundos que tiene un día, usados para convertir edades a días.
// TODO(core-lanzamiento): se consumirá en la Fase B2; retirar el allow entonces.
#[allow(dead_code)]
const MILLIS_PER_DAY: f64 = 86_400_000.0;

/// Nombre del fichero de base de datos dentro del directorio de configuración.
// TODO(core-lanzamiento): se consumirá en la Fase B2; retirar el allow entonces.
#[allow(dead_code)]
const DB_FILE_NAME: &str = "history.db";

/// Versión de esquema que aplican las migraciones al abrir la base de datos.
// TODO(core-lanzamiento): se consumirá en la Fase B2; retirar el allow entonces.
#[allow(dead_code)]
const SCHEMA_VERSION: i64 = 1;

/// Datos de uso acumulados de un ítem.
// TODO(core-lanzamiento): se consumirá en la Fase B2; retirar el allow entonces.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Usage {
    /// Número de lanzamientos registrados.
    pub launches: u64,
    /// Marca temporal (epoch, milisegundos) del primer lanzamiento.
    pub first_used: i64,
    /// Marca temporal (epoch, milisegundos) del último lanzamiento.
    pub last_used: i64,
}

/// Errores del módulo de historial.
// TODO(core-lanzamiento): se consumirá en la Fase B2; retirar el allow entonces.
#[allow(dead_code)]
#[derive(Debug)]
pub enum HistoryError {
    /// No se pudo resolver el directorio de configuración XDG.
    ConfigDirNotFound,
    /// Fallo de E/S al preparar el directorio de la base de datos.
    Io(std::io::Error),
    /// Fallo devuelto por SQLite.
    Sqlite(rusqlite::Error),
    /// El mutex que protege la conexión quedó envenenado.
    LockPoisoned,
}

impl fmt::Display for HistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigDirNotFound => {
                write!(f, "No se pudo resolver el directorio de configuración")
            }
            Self::Io(err) => write!(f, "Error de E/S en el historial: {err}"),
            Self::Sqlite(err) => write!(f, "Error de SQLite en el historial: {err}"),
            Self::LockPoisoned => write!(f, "La conexión del historial quedó bloqueada"),
        }
    }
}

impl std::error::Error for HistoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Sqlite(err) => Some(err),
            Self::ConfigDirNotFound | Self::LockPoisoned => None,
        }
    }
}

impl From<std::io::Error> for HistoryError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<rusqlite::Error> for HistoryError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Sqlite(err)
    }
}

/// Resultado de las operaciones del historial.
// TODO(core-lanzamiento): se consumirá en la Fase B2; retirar el allow entonces.
#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, HistoryError>;

/// Repositorio de historial de uso.
///
/// Desacopla el ranking de la persistencia: cualquier implementación sirve para
/// registrar lanzamientos y consultar puntuaciones.
// TODO(core-lanzamiento): se consumirá en la Fase B2; retirar el allow entonces.
#[allow(dead_code)]
pub trait HistoryRepository {
    /// Registra un lanzamiento del ítem `item_id` en el instante `now_millis`.
    fn record(&self, item_id: &str, now_millis: i64) -> Result<()>;

    /// Devuelve el uso acumulado de `item_id`, o `None` si nunca se lanzó.
    fn usage(&self, item_id: &str) -> Result<Option<Usage>>;

    /// Devuelve la puntuación de `item_id` en el instante `now_millis`.
    ///
    /// La puntuación es `ln(1 + launches) * decay`, con
    /// `decay = 0.5^(edad_dias / HALF_LIFE_DAYS)`. Un ítem desconocido puntúa
    /// `0.0`.
    fn score(&self, item_id: &str, now_millis: i64) -> Result<f64> {
        match self.usage(item_id)? {
            Some(usage) => Ok(usage_score(&usage, now_millis)),
            None => Ok(0.0),
        }
    }

    /// Devuelve todo el historial registrado como pares `(item_id, Usage)`.
    fn load_all(&self) -> Result<Vec<(String, Usage)>>;
}

/// Calcula la puntuación de uso de `usage` en el instante `now_millis`.
///
/// Combina frecuencia (`ln(1 + launches)`) y recencia
/// (`0.5^(edad_dias / HALF_LIFE_DAYS)`). Si `now_millis` es anterior al último
/// uso la edad se trata como cero, de modo que la puntuación nunca crece por
/// usar un reloj adelantado.
// TODO(core-lanzamiento): se consumirá en la Fase B2; retirar el allow entonces.
#[allow(dead_code)]
fn usage_score(usage: &Usage, now_millis: i64) -> f64 {
    let age_days = (now_millis - usage.last_used).max(0) as f64 / MILLIS_PER_DAY;
    let decay = 0.5_f64.powf(age_days / HALF_LIFE_DAYS);
    (1.0 + usage.launches as f64).ln() * decay
}

/// Construye un `Usage` a partir de una fila cuyas columnas `launches`,
/// `first_used` y `last_used` empiezan en `offset`.
// TODO(core-lanzamiento): se consumirá en la Fase B2; retirar el allow entonces.
#[allow(dead_code)]
fn usage_from_row(row: &rusqlite::Row<'_>, offset: usize) -> rusqlite::Result<Usage> {
    let launches: i64 = row.get(offset)?;
    Ok(Usage {
        launches: launches.max(0) as u64,
        first_used: row.get(offset + 1)?,
        last_used: row.get(offset + 2)?,
    })
}

/// Historial de uso persistido en una base de datos SQLite.
// TODO(core-lanzamiento): se consumirá en la Fase B2; retirar el allow entonces.
#[allow(dead_code)]
pub struct SqliteHistory {
    /// Conexión protegida por mutex: rusqlite no es sincronizado.
    conn: Mutex<Connection>,
}

// TODO(core-lanzamiento): se consumirá en la Fase B2; retirar el allow entonces.
#[allow(dead_code)]
impl SqliteHistory {
    /// Abre el historial en el directorio de configuración de la aplicación.
    ///
    /// Crea el directorio si no existe. La base de datos se guarda en
    /// `~/.config/lila/history.db`.
    pub fn open() -> Result<Self> {
        let project_dirs = ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_APPLICATION)
            .ok_or(HistoryError::ConfigDirNotFound)?;
        let config_dir = project_dirs.config_dir();
        if !config_dir.exists() {
            std::fs::create_dir_all(config_dir)?;
        }
        Self::open_at(config_dir.join(DB_FILE_NAME))
    }

    /// Abre el historial en la ruta indicada, creando el esquema si hace falta.
    ///
    /// Activa el modo WAL y aplica las migraciones idempotentes.
    pub fn open_at<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        debug!("directorio de historial: {:?}", path);
        let conn = Connection::open(path)?;
        let journal_mode: String =
            conn.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
        debug!("journal_mode del historial: {journal_mode}");
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.migrate()?;
        Ok(store)
    }

    /// Bloquea la conexión, traduciendo el envenenamiento del mutex a error.
    fn connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|_| HistoryError::LockPoisoned)
    }

    /// Aplica las migraciones idempotentes y fija la versión de esquema.
    fn migrate(&self) -> Result<()> {
        let conn = self.connection()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS launcher_history (
                 item_id    TEXT PRIMARY KEY,
                 launches   INTEGER NOT NULL DEFAULT 0,
                 first_used INTEGER NOT NULL,
                 last_used  INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_launcher_history_last_used
                 ON launcher_history(last_used);
             CREATE TABLE IF NOT EXISTS schema_version (
                 version INTEGER NOT NULL
             );",
        )?;
        let current: Option<i64> = conn
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
                row.get(0)
            })
            .optional()?;
        match current {
            None => {
                conn.execute(
                    "INSERT INTO schema_version (version) VALUES (?1)",
                    [SCHEMA_VERSION],
                )?;
            }
            Some(version) if version < SCHEMA_VERSION => {
                conn.execute("UPDATE schema_version SET version = ?1", [SCHEMA_VERSION])?;
            }
            Some(_) => {}
        }
        Ok(())
    }
}

impl HistoryRepository for SqliteHistory {
    fn record(&self, item_id: &str, now_millis: i64) -> Result<()> {
        let conn = self.connection()?;
        conn.execute(
            "INSERT INTO launcher_history (item_id, launches, first_used, last_used)
             VALUES (?1, 1, ?2, ?2)
             ON CONFLICT(item_id) DO UPDATE SET
                 launches  = launcher_history.launches + 1,
                 last_used = excluded.last_used",
            rusqlite::params![item_id, now_millis],
        )?;
        Ok(())
    }

    fn usage(&self, item_id: &str) -> Result<Option<Usage>> {
        let conn = self.connection()?;
        let usage = conn
            .query_row(
                "SELECT launches, first_used, last_used
                 FROM launcher_history
                 WHERE item_id = ?1",
                [item_id],
                |row| usage_from_row(row, 0),
            )
            .optional()?;
        Ok(usage)
    }

    fn load_all(&self) -> Result<Vec<(String, Usage)>> {
        let conn = self.connection()?;
        let mut stmt = conn.prepare(
            "SELECT item_id, launches, first_used, last_used
             FROM launcher_history",
        )?;
        let rows = stmt.query_map([], |row| {
            let usage = usage_from_row(row, 1)?;
            Ok((row.get(0)?, usage))
        })?;
        let mut all = Vec::new();
        for row in rows {
            all.push(row?);
        }
        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::OptionalExtension;
    use tempfile::TempDir;

    /// Instante fijo para que los tests sean deterministas.
    const T0: i64 = 1_700_000_000_000;
    /// Milisegundos que tiene un día.
    const DAY_MILLIS: i64 = 86_400_000;

    /// Crea un historial temporal y devuelve su `TempDir` para que viva en el test.
    fn temp_history() -> (TempDir, SqliteHistory) {
        let dir = TempDir::new().expect("No se pudo crear el directorio temporal");
        let store = SqliteHistory::open_at(dir.path().join("history.db"))
            .expect("No se pudo abrir el historial temporal");
        (dir, store)
    }

    /// [TDD-RED-01] Registrar un lanzamiento incrementa el contador y actualiza
    /// la marca de último uso.
    #[test]
    fn record_increments_launches_and_updates_last_used() {
        let (_dir, store) = temp_history();

        store.record("firefox.desktop", T0).expect("record falló");
        store
            .record("firefox.desktop", T0 + 1_000)
            .expect("record falló");

        let usage = store
            .usage("firefox.desktop")
            .expect("usage falló")
            .expect("debería existir el ítem");
        assert_eq!(usage.launches, 2);
        assert_eq!(usage.last_used, T0 + 1_000);
    }

    /// [TDD-RED-02] El segundo lanzamiento conserva la marca del primero.
    #[test]
    fn record_preserves_first_used_on_second_launch() {
        let (_dir, store) = temp_history();

        store.record("firefox.desktop", T0).expect("record falló");
        store
            .record("firefox.desktop", T0 + DAY_MILLIS)
            .expect("record falló");

        let usage = store
            .usage("firefox.desktop")
            .expect("usage falló")
            .expect("debería existir el ítem");
        assert_eq!(usage.first_used, T0);
        assert_eq!(usage.last_used, T0 + DAY_MILLIS);
    }

    /// [TDD-RED-03] Un ítem sin lanzamientos puntúa cero.
    #[test]
    fn score_is_zero_for_unknown_item() {
        let (_dir, store) = temp_history();

        let score = store.score("desconocido.desktop", T0).expect("score falló");
        assert_eq!(score, 0.0);
    }

    /// [TDD-RED-04] La puntuación decae con la edad del último uso.
    #[test]
    fn score_decays_with_age() {
        let (_dir, store) = temp_history();
        store.record("firefox.desktop", T0).expect("record falló");

        let recent = store.score("firefox.desktop", T0).expect("score falló");
        let aged = store
            .score("firefox.desktop", T0 + 30 * DAY_MILLIS)
            .expect("score falló");

        assert!(recent > aged, "esperado {recent} > {aged}");
    }

    /// [TDD-RED-05] El historial se conserva al reabrir la misma ruta.
    #[test]
    fn history_survives_reopening_same_path() {
        let dir = TempDir::new().expect("No se pudo crear el directorio temporal");
        let path = dir.path().join("history.db");

        {
            let store = SqliteHistory::open_at(&path).expect("No se pudo abrir el historial");
            store.record("firefox.desktop", T0).expect("record falló");
        }

        let reopened = SqliteHistory::open_at(&path).expect("No se pudo reabrir el historial");
        let usage = reopened
            .usage("firefox.desktop")
            .expect("usage falló")
            .expect("el historial debería conservarse");
        assert_eq!(usage.launches, 1);
        assert_eq!(usage.last_used, T0);
    }

    /// [TDD-RED-06] Abrir dos veces no falla y deja la versión de esquema actual.
    #[test]
    fn migrations_are_idempotent_and_store_current_schema_version() {
        let dir = TempDir::new().expect("No se pudo crear el directorio temporal");
        let path = dir.path().join("history.db");

        {
            let _store = SqliteHistory::open_at(&path).expect("No se pudo abrir el historial");
        }
        let store = SqliteHistory::open_at(&path).expect("No se pudo reabrir el historial");

        let conn = store.conn.lock().expect("No se pudo bloquear la conexión");
        let version: Option<i64> = conn
            .query_row("SELECT version FROM schema_version", [], |row| row.get(0))
            .optional()
            .expect("No se pudo leer schema_version");
        assert_eq!(version, Some(SCHEMA_VERSION));
    }

    /// [TDD-RED-08] Abrir el historial deja la base de datos en modo WAL.
    #[test]
    fn open_at_enables_wal_journal_mode() {
        let (_dir, store) = temp_history();

        let conn = store.conn.lock().expect("No se pudo bloquear la conexión");
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("No se pudo leer journal_mode");

        assert_eq!(mode.to_lowercase(), "wal");
    }

    /// [TDD-RED-07] `load_all` devuelve todo lo registrado.
    #[test]
    fn load_all_returns_recorded_items() {
        let (_dir, store) = temp_history();

        store.record("firefox.desktop", T0).expect("record falló");
        store
            .record("alacritty.desktop", T0 + 1_000)
            .expect("record falló");

        let mut all = store.load_all().expect("load_all falló");
        all.sort_by(|a, b| a.0.cmp(&b.0));

        assert_eq!(all.len(), 2);
        assert_eq!(all[0].0, "alacritty.desktop");
        assert_eq!(all[0].1.launches, 1);
        assert_eq!(all[1].0, "firefox.desktop");
        assert_eq!(all[1].1.last_used, T0);
    }
}
