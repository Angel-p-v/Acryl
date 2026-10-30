//! Apertura de base de datos SQLite.

use rusqlite::{Connection, Result as RusqliteResult};

use crate::db::migrations::{MIGRATIONS, Migration};

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

pub fn close_db(conn: Connection) {
    let _ = conn.execute_batch("PRAGMA optimize");
    let _ = conn.close();
}

/// Ejecuta una migración completa.
///
/// `execute_batch` respeta los bloques `BEGIN ... END` de los triggers de SQLite,
/// algo que dividir el SQL por `;` rompería. Si la migración falla porque una base
/// de datos antigua ya contiene alguna columna (caso de migraciones previas a
/// `schema_migrations`), se reintenta statement a statement tolerando únicamente
/// los errores idempotentes esperados.
fn run_migration(conn: &mut Connection, migration: &Migration) -> RusqliteResult<()> {
    match conn.execute_batch(migration.sql) {
        Ok(()) => Ok(()),
        Err(batch_error) => {
            // Los triggers contienen puntos y coma en su cuerpo; no se pueden
            // reintentar con un split ingenuo sin romperlos.
            if migration
                .sql
                .to_ascii_uppercase()
                .contains("CREATE TRIGGER")
            {
                return Err(batch_error);
            }

            for stmt in migration
                .sql
                .split(';')
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                if let Err(error) = conn.execute(stmt, []) {
                    if is_idempotent_migration_error(&error) {
                        continue;
                    }
                    return Err(error);
                }
            }
            Ok(())
        }
    }
}

fn is_idempotent_migration_error(error: &rusqlite::Error) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    message.contains("duplicate column name") || message.contains("already exists")
}

fn run_migrations(
    conn: &mut Connection,
    migrations: &[Migration],
    _db_path: &str,
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

    for migration in migrations {
        if applied_set.contains(&migration.version) {
            continue;
        }

        run_migration(conn, migration)?;
        conn.prepare("INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)")?
            .execute(rusqlite::params![migration.version, migration.name])?;
    }

    Ok(())
}

pub fn resolve_db_path(db_path: &str) -> std::path::PathBuf {
    let path = std::path::Path::new(db_path);
    if path.is_relative() && db_path != ":memory:" {
        crate::config::default_data_dir().join(db_path)
    } else {
        path.to_path_buf()
    }
}
