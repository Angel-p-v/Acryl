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

    // Verificar integridad de la base de datos al abrirla. Si la copia de
    // seguridad o el archivo están corruptos, fallar rápido en lugar de
    // seguir trabajando con datos posiblemente dañados.
    let check: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if check != "ok" {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
            Some(format!("base de datos corrupta: {}", check)),
        ));
    }

    Ok(conn)
}

/// Ejecuta mantenimiento de la base de datos: `VACUUM` para recuperar espacio
/// y `ANALYZE` para actualizar estadísticas del planificador de consultas.
/// `VACUUM` requiere que no haya transacciones activas, por lo que se ejecuta
/// en una conexión dedicada y se cierra inmediatamente.
pub fn vacuum_database(db_path: &str) -> RusqliteResult<()> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("VACUUM; ANALYZE;")?;
    Ok(())
}

pub fn close_db(conn: Connection) {
    let _ = conn.execute_batch("PRAGMA optimize");
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)");
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

#[cfg(test)]
mod tests {
    use super::*;

    fn migration(version: i32, name: &'static str, sql: &'static str) -> Migration {
        Migration { version, name, sql }
    }

    fn memory_db() -> Connection {
        Connection::open_in_memory().expect("abrir BD en memoria")
    }

    #[test]
    fn resolve_db_path_respeta_rutas_absolutas_y_memoria() {
        assert_eq!(
            resolve_db_path("/tmp/acryl/test.db"),
            std::path::PathBuf::from("/tmp/acryl/test.db")
        );
        assert_eq!(
            resolve_db_path(":memory:"),
            std::path::PathBuf::from(":memory:")
        );

        let relative = resolve_db_path("sub/acryl.db");
        assert!(relative.is_absolute(), "las rutas relativas se anclan");
        assert!(relative.ends_with("sub/acryl.db"));
    }

    #[test]
    fn run_migrations_aplica_cada_version_una_sola_vez() {
        let mut conn = memory_db();
        let migrations = [
            migration(1, "uno", "CREATE TABLE demo (id INTEGER);"),
            migration(2, "dos", "ALTER TABLE demo ADD COLUMN name TEXT;"),
        ];

        run_migrations(&mut conn, &migrations, ":memory:").expect("aplicar migraciones");
        run_migrations(&mut conn, &migrations, ":memory:").expect("reaplicar es idempotente");

        let applied: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("contar migraciones");
        assert_eq!(applied, 2);
    }

    #[test]
    fn run_migration_tolera_tablas_y_columnas_ya_existentes() {
        let mut conn = memory_db();
        conn.execute_batch(
            "CREATE TABLE demo (id INTEGER);
             ALTER TABLE demo ADD COLUMN name TEXT;",
        )
        .expect("preparar esquema previo");

        let duplicated = migration(
            3,
            "duplicada",
            "CREATE TABLE demo (id INTEGER);
             ALTER TABLE demo ADD COLUMN name TEXT;",
        );
        run_migration(&mut conn, &duplicated).expect("tolera errores idempotentes");
    }

    #[test]
    fn run_migration_propaga_errores_no_idempotentes() {
        let mut conn = memory_db();
        let broken = migration(
            4,
            "rota",
            "CREATE TABLE demo (id INTEGER);
             ESTO NO ES SQL;",
        );
        assert!(run_migration(&mut conn, &broken).is_err());
    }

    #[test]
    fn run_migration_no_reintenta_migraciones_con_triggers() {
        let mut conn = memory_db();
        conn.execute_batch("CREATE TABLE demo (id INTEGER);")
            .expect("preparar tabla");
        let trigger = migration(
            5,
            "trigger",
            "CREATE TABLE demo (id INTEGER);
             CREATE TRIGGER demo_trg AFTER INSERT ON demo BEGIN SELECT 1; END;",
        );
        assert!(
            run_migration(&mut conn, &trigger).is_err(),
            "no se divide el cuerpo de un trigger"
        );
    }

    #[test]
    fn is_idempotent_migration_error_detecta_los_mensajes_conocidos() {
        let conn = memory_db();
        conn.execute_batch("CREATE TABLE demo (id INTEGER);")
            .expect("crear tabla");

        let exists = conn
            .execute_batch("CREATE TABLE demo (id INTEGER);")
            .expect_err("la tabla ya existe");
        assert!(is_idempotent_migration_error(&exists));

        let duplicate = conn
            .execute_batch("ALTER TABLE demo ADD COLUMN id INTEGER;")
            .expect_err("la columna ya existe");
        assert!(is_idempotent_migration_error(&duplicate));

        let other = conn
            .execute_batch("ESTO NO ES SQL;")
            .expect_err("error de sintaxis");
        assert!(!is_idempotent_migration_error(&other));
    }
}
