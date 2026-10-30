//! Tests de integración para la persistencia de medios.
//! Ejercitan migraciones, repositorios y servicios de dominio con BD SQLite temporal.

use acryl::db::connection::open_db;
use acryl::db::file_repo::{FileRepository, MediaFileInput};
use acryl::db::media_repo::{ListMediaOptions, MediaInput, MediaRepository, SearchMediaOptions};
use std::path::PathBuf;
use tempfile::TempDir;

/// Crea una BD temporal vacía y devuelve la ruta y el directorio temporal.
fn temp_db() -> (PathBuf, TempDir) {
    let dir = TempDir::new().expect("crear directorio temporal");
    let path = dir.path().join("test.db");
    (path, dir)
}

#[test]
fn migraciones_crean_esquema_inicial() {
    let (db_path, _dir) = temp_db();
    let conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .filter_map(Result::ok)
        .collect();

    assert!(tables.contains(&"media".to_string()));
    assert!(tables.contains(&"media_files".to_string()));
    assert!(tables.contains(&"index_runs".to_string()));
    assert!(tables.contains(&"playback_history".to_string()));
    assert!(tables.contains(&"schema_migrations".to_string()));
    assert!(tables.contains(&"media_fts".to_string()));
}

#[test]
fn upsert_y_list_media() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    let (m1, m2) = {
        let mut repo = MediaRepository::new(&mut conn);
        let m1 = repo
            .upsert(&MediaInput {
                r#type: "movie".to_string(),
                title: "Matrix".to_string(),
                sort_title: "matrix".to_string(),
                year: Some(1999),
                folder_path: "/media/matrix".to_string(),
                parent_id: None,
            })
            .expect("upsert media 1");
        let m2 = repo
            .upsert(&MediaInput {
                r#type: "series".to_string(),
                title: "Breaking Bad".to_string(),
                sort_title: "breaking bad".to_string(),
                year: Some(2008),
                folder_path: "/media/bb".to_string(),
                parent_id: None,
            })
            .expect("upsert media 2");
        (m1, m2)
    };

    assert_eq!(m1.id, 1);
    assert_eq!(m2.id, 2);

    let all = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.list(ListMediaOptions {
            r#type: None,
            limit: Some(50),
            offset: Some(0),
            order_by: None,
        })
        .expect("list all")
    };
    assert_eq!(all.len(), 2);

    let movies = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.list(ListMediaOptions {
            r#type: Some("movie".to_string()),
            limit: Some(50),
            offset: Some(0),
            order_by: None,
        })
        .expect("list movies")
    };
    assert_eq!(movies.len(), 1);
    assert_eq!(movies[0].title, "Matrix");

    let series = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.list(ListMediaOptions {
            r#type: Some("series".to_string()),
            limit: Some(50),
            offset: Some(0),
            order_by: None,
        })
        .expect("list series")
    };
    assert_eq!(series.len(), 1);
    assert_eq!(series[0].title, "Breaking Bad");
}

#[test]
fn upsert_es_idempotente_por_folder_path() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    let m1 = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.upsert(&MediaInput {
            r#type: "movie".to_string(),
            title: "Matrix v1".to_string(),
            sort_title: "matrix".to_string(),
            year: None,
            folder_path: "/media/matrix".to_string(),
            parent_id: None,
        })
        .expect("upsert 1")
    };

    let m2 = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.upsert(&MediaInput {
            r#type: "movie".to_string(),
            title: "Matrix v2".to_string(),
            sort_title: "matrix".to_string(),
            year: Some(1999),
            folder_path: "/media/matrix".to_string(),
            parent_id: None,
        })
        .expect("upsert 2")
    };

    assert_eq!(m1.id, m2.id, "debe ser el mismo registro");

    let all = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.list(ListMediaOptions {
            r#type: None,
            limit: Some(50),
            offset: Some(0),
            order_by: None,
        })
        .expect("list")
    };
    assert_eq!(all.len(), 1, "no debe haber duplicados");
    assert_eq!(all[0].title, "Matrix v2", "debe actualizar el título");
}

#[test]
fn archivos_por_medio() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    let media = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.upsert(&MediaInput {
            r#type: "movie".to_string(),
            title: "Película".to_string(),
            sort_title: "pelicula".to_string(),
            year: None,
            folder_path: "/media/pelicula".to_string(),
            parent_id: None,
        })
        .expect("upsert media")
    };

    let inserted = {
        let mut file_repo = FileRepository::new(&mut conn);
        file_repo
            .upsert_many(&[
                MediaFileInput {
                    media_id: media.id,
                    season: None,
                    episode: None,
                    title: Some("parte 1".to_string()),
                    file_path: "/media/pelicula/video1.mp4".to_string(),
                    size_bytes: Some(100),
                    mtime_ms: None,
                },
                MediaFileInput {
                    media_id: media.id,
                    season: None,
                    episode: None,
                    title: Some("parte 2".to_string()),
                    file_path: "/media/pelicula/video2.mp4".to_string(),
                    size_bytes: Some(200),
                    mtime_ms: None,
                },
            ])
            .expect("upsert_many files")
    };
    assert_eq!(inserted, 2);

    let files = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.list_files_by_media(media.id)
            .expect("list files by media")
    };
    assert_eq!(files.len(), 2);
}

#[test]
fn toggle_favorite_y_watched() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    let media = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.upsert(&MediaInput {
            r#type: "movie".to_string(),
            title: "Test".to_string(),
            sort_title: "test".to_string(),
            year: None,
            folder_path: "/media/test".to_string(),
            parent_id: None,
        })
        .expect("upsert")
    };

    assert!(!media.is_favorite);
    assert!(!media.is_watched);

    let fav = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.toggle_favorite(media.id).expect("toggle favorite")
    };
    assert!(fav);

    let fav_off = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.toggle_favorite(media.id).expect("toggle favorite off")
    };
    assert!(!fav_off);

    let wat = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.toggle_watched(media.id).expect("toggle watched")
    };
    assert!(wat);
}

#[test]
fn find_by_id_devuelve_none_si_no_existe() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");
    let mut repo = MediaRepository::new(&mut conn);

    let encontrado = repo.find_by_id(999).expect("find by id missing");
    assert!(encontrado.is_none());
}

#[test]
fn record_playback_inserta_historial() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    let media = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.upsert(&MediaInput {
            r#type: "movie".to_string(),
            title: "Test".to_string(),
            sort_title: "test".to_string(),
            year: None,
            folder_path: "/media/test".to_string(),
            parent_id: None,
        })
        .expect("upsert")
    };

    {
        let mut repo = MediaRepository::new(&mut conn);
        repo.record_playback(media.id, 120.5, 3600.0)
            .expect("record playback");
    }

    let count: i64 = {
        let conn2 = open_db(db_path.to_str().unwrap()).expect("abrir BD");
        conn2
            .prepare("SELECT COUNT(*) FROM playback_history WHERE media_id = ?1")
            .unwrap()
            .query_row([media.id], |row| row.get::<_, i64>(0))
            .expect("count playback")
    };
    assert_eq!(count, 1);
}

#[test]
fn get_details_devuelve_media_con_archivos() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    let media = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.upsert(&MediaInput {
            r#type: "movie".to_string(),
            title: "Con archivos".to_string(),
            sort_title: "con archivos".to_string(),
            year: None,
            folder_path: "/media/con-archivos".to_string(),
            parent_id: None,
        })
        .expect("upsert media")
    };

    {
        let mut file_repo = FileRepository::new(&mut conn);
        file_repo
            .upsert_many(&[MediaFileInput {
                media_id: media.id,
                season: None,
                episode: None,
                title: Some("video.mp4".to_string()),
                file_path: "/media/con-archivos/video.mp4".to_string(),
                size_bytes: Some(50),
                mtime_ms: None,
            }])
            .expect("upsert_many");
    }

    let details = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.get_details(media.id).expect("get details")
    };

    let details = details.expect("details debe existir");
    assert_eq!(details.media.id, media.id);
    assert_eq!(details.files.len(), 1);
    assert_eq!(details.files[0].title.as_deref(), Some("video.mp4"));
}

#[test]
fn busqueda_encuentra_medios_recien_insertados() {
    let (db_path, _dir) = temp_db();

    {
        let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");
        let mut repo = MediaRepository::new(&mut conn);
        repo.upsert(&MediaInput {
            r#type: "series".to_string(),
            title: "Black Torch".to_string(),
            sort_title: "black torch".to_string(),
            year: None,
            folder_path: "/media/black-torch".to_string(),
            parent_id: None,
        })
        .expect("upsert media");
    }

    let mut conn = open_db(db_path.to_str().unwrap()).expect("reabrir BD");
    let mut repo = MediaRepository::new(&mut conn);
    let results = repo
        .search(
            "Black Torch",
            SearchMediaOptions {
                r#type: None,
                limit: Some(10),
            },
        )
        .expect("buscar media");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Black Torch");
}

#[test]
fn busqueda_usa_like_cuando_el_indice_fts_esta_vacio() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    {
        let mut repo = MediaRepository::new(&mut conn);
        repo.upsert(&MediaInput {
            r#type: "series".to_string(),
            title: "Black Torch".to_string(),
            sort_title: "black torch".to_string(),
            year: None,
            folder_path: "/media/black-torch".to_string(),
            parent_id: None,
        })
        .expect("upsert media");
    }

    // Simula un índice FTS vacío para forzar el camino LIKE.
    conn.execute("INSERT INTO media_fts(media_fts) VALUES ('delete-all')", [])
        .expect("vaciar índice FTS");

    let mut repo = MediaRepository::new(&mut conn);
    let results = repo
        .search(
            "black",
            SearchMediaOptions {
                r#type: None,
                limit: Some(10),
            },
        )
        .expect("buscar media con fallback LIKE");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Black Torch");
}

#[test]
fn migracion_repara_triggers_fts_de_una_bd_existente() {
    let (db_path, _dir) = temp_db();

    {
        let conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");
        conn.execute_batch(
            "DROP TRIGGER IF EXISTS media_fts_insert;
             DROP TRIGGER IF EXISTS media_fts_delete;
             DROP TRIGGER IF EXISTS media_fts_update;
             DELETE FROM schema_migrations WHERE version = 5;",
        )
        .expect("simular BD antigua sin triggers FTS");

        conn.execute(
            "INSERT INTO media (type, title, sort_title, folder_path)
             VALUES ('series', 'Legacy Show', 'legacy show', '/media/legacy')",
            [],
        )
        .expect("insertar medio sin trigger FTS");
    }

    // Al reabrir, la migración 5 debe recrear los triggers y reconstruir el índice.
    let mut conn = open_db(db_path.to_str().unwrap()).expect("reabrir BD");
    let triggers: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'trigger' AND name LIKE 'media_fts_%'",
            [],
            |row| row.get(0),
        )
        .expect("contar triggers");
    assert_eq!(triggers, 3);

    let mut repo = MediaRepository::new(&mut conn);
    let results = repo
        .search(
            "legacy",
            SearchMediaOptions {
                r#type: None,
                limit: Some(10),
            },
        )
        .expect("buscar medio antiguo");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Legacy Show");
}

#[test]
fn update_metadata_actualiza_anio_poster_y_sinopsis() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    let media = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.upsert(&MediaInput {
            r#type: "series".to_string(),
            title: "Solo Leveling".to_string(),
            sort_title: "solo leveling".to_string(),
            year: None,
            folder_path: "/media/solo-leveling".to_string(),
            parent_id: None,
        })
        .expect("upsert media")
    };

    {
        let mut repo = MediaRepository::new(&mut conn);
        repo.update_metadata(
            media.id,
            Some(2024),
            Some("https://example.com/poster.jpg"),
            Some("Sinopsis de prueba"),
        )
        .expect("actualizar metadatos");
    }

    let updated = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.find_by_id(media.id)
            .expect("buscar media")
            .expect("media existente")
    };

    assert_eq!(updated.year, Some(2024));
    assert_eq!(
        updated.poster_path.as_deref(),
        Some("https://example.com/poster.jpg")
    );
    assert_eq!(updated.synopsis.as_deref(), Some("Sinopsis de prueba"));
    assert!(updated.metadata_fetched_at.is_some());
}

#[test]
fn overwrite_metadata_reemplaza_campos_y_actualiza_fts() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    let media_id = {
        let mut repo = MediaRepository::new(&mut conn);
        let media = repo
            .upsert(&MediaInput {
                r#type: "series".to_string(),
                title: "Zafiro Antiguo".to_string(),
                sort_title: "zafiro antiguo".to_string(),
                year: Some(2001),
                folder_path: "/media/zafiro".to_string(),
                parent_id: None,
            })
            .expect("upsert media");
        repo.update_metadata(
            media.id,
            Some(2001),
            Some("https://example.com/old.jpg"),
            Some("Sinopsis antigua"),
        )
        .expect("guardar metadatos antiguos");
        media.id
    };

    {
        let mut repo = MediaRepository::new(&mut conn);
        repo.overwrite_metadata(
            media_id,
            "Esmeralda Nueva",
            "esmeralda nueva",
            Some(2025),
            Some("https://example.com/new.jpg"),
            Some("Sinopsis nueva"),
        )
        .expect("sobrescribir metadatos");
    }

    let mut repo = MediaRepository::new(&mut conn);
    let media = repo
        .find_by_id(media_id)
        .expect("buscar media")
        .expect("media existente");
    assert_eq!(media.title, "Esmeralda Nueva");
    assert_eq!(media.sort_title, "esmeralda nueva");
    assert_eq!(media.year, Some(2025));
    assert_eq!(
        media.poster_path.as_deref(),
        Some("https://example.com/new.jpg")
    );
    assert_eq!(media.synopsis.as_deref(), Some("Sinopsis nueva"));

    let new_results = repo
        .search(
            "Esmeralda",
            SearchMediaOptions {
                r#type: None,
                limit: Some(10),
            },
        )
        .expect("buscar por el título nuevo");
    assert_eq!(new_results.len(), 1);
    assert_eq!(new_results[0].id, media_id);

    let old_results = repo
        .search(
            "Zafiro",
            SearchMediaOptions {
                r#type: None,
                limit: Some(10),
            },
        )
        .expect("buscar por el título antiguo");
    assert!(old_results.is_empty());
}
