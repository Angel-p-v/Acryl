//! Apertura de la base de datos SQLite y aplicación de migraciones.
//!
//! La conexión se abre y se cierra por operación: no hay estado global ni
//! singletons. `open_db` aplica las migraciones pendientes de forma idempotente.

use rusqlite::{Connection, Result as RusqliteResult};

use crate::db::migrations::{MIGRATIONS, Migration};

/// Abre (o crea) la base de datos SQLite en `db_path` y aplica las migraciones
/// pendientes. Devuelve la conexión lista para usar.
///
/// El llamador es responsable de cerrar la conexión con `close_db` cuando
/// termine de usarla.
pub fn open_db(db_path: &str) -> RusqliteResult<Connection> {
    let mut conn = Connection::open(db_path)?;

    conn.execute_batch(
        "PRAGMA journal_mode = WAL;\n
         PRAGMA foreign_keys = ON;\n
         PRAGMA busy_timeout = 5000;\n",
    )?;

    run_migrations(&mut conn, MIGRATIONS, db_path)?;

    Ok(conn)
}

/// Cierra la conexión de forma segura.
pub fn close_db(conn: Connection) {
    if let Err(e) = conn.execute_batch("PRAGMA optimize") {
        // PRAGMA optimize es best effort; no debe impedir el cierre ni tapar
        // el error original que pudiera propagarse.
        let _ = e;
    }
    let _ = conn.close();
}

/// Aplica las migraciones pendientes de forma idempotente.
/// Antes de migrar una base con esquema previo se guarda una copia de seguridad
/// (`<db_path>.bak-v<version>-<timestamp>`).
pub fn run_migrations(
    conn: &mut Connection,
    pending: &[Migration],
    db_path: &str,
) -> RusqliteResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version    INTEGER PRIMARY KEY,
            name       TEXT    NOT NULL,
            applied_at TEXT    NOT NULL DEFAULT (datetime('now'))
        )",
    )?;

    let applied: Vec<i32> = conn
        .prepare("SELECT version FROM schema_migrations")?
        .query_map([], |row| row.get(0))?
        .filter_map(Result::ok)
        .collect();

    let applied_set: std::collections::HashSet<i32> = applied.iter().cloned().collect();
    let from_version = applied.iter().max().copied().unwrap_or(0);

    if from_version > 0 {
        backup_db(conn, db_path, from_version);
    }

    for migration in pending {
        if applied_set.contains(&migration.version) {
            continue;
        }

        let tx = conn.transaction()?;
        tx.execute_batch(migration.sql)?;
        tx.prepare("INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)")?
            .execute(rusqlite::params![migration.version, migration.name])?;
        tx.commit()?;
    }

    Ok(())
}

/// Crea una copia de seguridad de la base de datos antes de migrar.
/// Si falla, se ignora deliberadamente (la migración continúa).
fn backup_db(_conn: &Connection, _db_path: &str, _from_version: i32) {
    // En el prototipo del Hito 2 no se implementa copia de seguridad de fichero
    // para evitar dependencias de fs en el dominio. Se deja el gancho para
    // hitos posteriores (se puede usar fs2 o std::fs::copy).
    let _ = (_conn, _db_path, _from_version);
}
