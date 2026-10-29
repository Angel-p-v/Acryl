use crate::media::Media;
use rusqlite::{params, Connection, Result as RusqliteResult};

/// Abre (o crea) la base de datos SQLite y aplica el esquema base (v1).
/// La conexión se abre y se cierra por comando: no hay estado global.
pub fn open_db(db_path: &str) -> RusqliteResult<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;\n
         PRAGMA foreign_keys = ON;\n
         PRAGMA busy_timeout = 5000;\n
         CREATE TABLE IF NOT EXISTS schema_migrations (\n
             version    INTEGER PRIMARY KEY,\n
             name       TEXT    NOT NULL,\n
             applied_at TEXT    NOT NULL DEFAULT (datetime('now'))\n
         );\n
         CREATE TABLE IF NOT EXISTS media (\n
             id          INTEGER PRIMARY KEY,\n
             type        TEXT    NOT NULL CHECK (type IN ('series', 'anime', 'movie')),\n
             title       TEXT    NOT NULL,\n
             sort_title  TEXT    NOT NULL,\n
             year        INTEGER,\n
             folder_path TEXT    NOT NULL UNIQUE,\n
             created_at  TEXT    NOT NULL DEFAULT (datetime('now')),\n
             updated_at  TEXT    NOT NULL DEFAULT (datetime('now'))\n
         );",
    )?;
    Ok(conn)
}

/// Inserta ejemplos de medios si la tabla esta vacia (solo prototipo Hito 1).
/// Se remplazara por el indexador real en hitos posteriores.
fn seed_if_empty(conn: &Connection) -> RusqliteResult<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM media", [], |row| row.get(0))?;
    if count == 0 {
        let examples = [
            ("movie", "Interesting", "interesting", Some(2014), "/media/movies/interesting"),
            ("series", "Breaking Bad", "breaking bad", Some(2008), "/media/series/breaking-bad"),
            ("anime", "Cowboy Bebop", "cowboy bebop", Some(1998), "/media/anime/cowboy-bebop"),
        ];
        let mut stmt = conn.prepare(
            "INSERT INTO media (type, title, sort_title, year, folder_path)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        for (r#type, title, sort_title, year, folder_path) in examples {
            stmt.execute(params![r#type, title, sort_title, year, folder_path])?;
        }
    }
    Ok(())
}

/// Lista medios desde SQLite, con filtro opcional por titulo (busqueda LIKE).
pub fn list_media(db_path: &str, query: Option<&str>) -> RusqliteResult<Vec<Media>> {
    let conn = open_db(db_path)?;
    seed_if_empty(&conn)?;

    let mut sql = String::from(
        "SELECT id, type, title, sort_title, year, folder_path FROM media",
    );
    if let Some(q) = query {
        sql.push_str(" WHERE title LIKE ?1 ORDER BY sort_title");
        let like = format!("%{}%", q);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![like], map_media)?;
        rows.collect()
    } else {
        sql.push_str(" ORDER BY sort_title");
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], map_media)?;
        rows.collect()
    }
}

fn map_media(row: &rusqlite::Row<'_>) -> RusqliteResult<Media> {
    Ok(Media {
        id: row.get(0)?,
        r#type: row.get(1)?,
        title: row.get(2)?,
        sort_title: row.get(3)?,
        year: row.get(4)?,
        folder_path: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_media_devuelve_ejemplos_cuando_tabla_vacia() {
        let medios = list_media(":memory:", None).expect("debe listar medios");
        assert!(!medios.is_empty(), "debe haber al menos un medio de ejemplo");
        let titulos: Vec<&str> = medios.iter().map(|m| m.title.as_str()).collect();
        assert!(titulos.contains(&"Breaking Bad"));
        assert!(titulos.contains(&"Cowboy Bebop"));
        assert!(titulos.contains(&"Interesting"));
    }

    #[test]
    fn list_media_filtra_por_query() {
        let todos = list_media(":memory:", None).expect("debe listar medios");
        assert!(!todos.is_empty());

        let filtrados = list_media(":memory:", Some("bebop"))
            .expect("debe filtrar por query");
        assert_eq!(filtrados.len(), 1);
        assert_eq!(filtrados[0].title, "Cowboy Bebop");

        let sin_coincidencia = list_media(":memory:", Some("zzz-nada"))
            .expect("debe devolver vacio");
        assert!(sin_coincidencia.is_empty());
    }
}
