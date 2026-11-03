//! Tests de integración para la persistencia de medios.
//! Ejercitan migraciones, repositorios y servicios de dominio con BD SQLite temporal.

use acryl::core::player::{
    PlayerWindowMode, build_mpv_launch_args_for_playback, resolve_track_preference,
};
use acryl::db::connection::open_db;
use acryl::db::file_repo::{FileRepository, MediaFileInput, TrackPreference, TrackSelection};
use acryl::db::media_repo::{
    ListMediaOptions, MediaInput, MediaMetadataInput, MediaRepository, SearchMediaOptions,
};
use rusqlite::params;
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
        repo.record_playback(media.id, "/media/test/video.mp4", 120.5, 3600.0)
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
fn migracion_anade_banner_path_a_una_bd_existente() {
    let (db_path, _dir) = temp_db();

    {
        let conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");
        // Simula una base de datos anterior a la migración 6.
        conn.execute_batch(
            "ALTER TABLE media DROP COLUMN banner_path;
             DELETE FROM schema_migrations WHERE version = 6;",
        )
        .expect("simular BD antigua sin banner_path");
    }

    let conn = open_db(db_path.to_str().unwrap()).expect("reabrir BD");
    let mut has_banner = false;
    for column in conn
        .prepare("PRAGMA table_info(media)")
        .expect("preparar pragma")
        .query_map([], |row| row.get::<_, String>(1))
        .expect("consultar columnas")
    {
        if column.expect("columna") == "banner_path" {
            has_banner = true;
        }
    }

    assert!(has_banner, "la migración debe añadir banner_path");
}

#[test]
fn migracion_anade_origen_de_metadatos_y_marca_de_episodio() {
    let (db_path, _dir) = temp_db();

    {
        let conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");
        // Simula una base de datos anterior a la migración 10.
        conn.execute_batch(
            "DROP INDEX IF EXISTS idx_media_files_episode_image_pending;
             ALTER TABLE media DROP COLUMN metadata_source_id;
             ALTER TABLE media DROP COLUMN categories_fetched_at;
             ALTER TABLE media_files DROP COLUMN episode_image_checked_at;
             DELETE FROM schema_migrations WHERE version = 10;",
        )
        .expect("simular BD antigua sin columnas de metadatos");
    }

    let conn = open_db(db_path.to_str().unwrap()).expect("reabrir BD");
    let mut media_columns = Vec::new();
    for column in conn
        .prepare("PRAGMA table_info(media)")
        .expect("preparar pragma media")
        .query_map([], |row| row.get::<_, String>(1))
        .expect("consultar columnas media")
    {
        media_columns.push(column.expect("columna media"));
    }
    assert!(media_columns.contains(&"metadata_source_id".to_string()));
    assert!(media_columns.contains(&"categories_fetched_at".to_string()));

    let file_columns: Vec<String> = conn
        .prepare("PRAGMA table_info(media_files)")
        .expect("preparar pragma media_files")
        .query_map([], |row| row.get::<_, String>(1))
        .expect("consultar columnas media_files")
        .filter_map(Result::ok)
        .collect();
    assert!(file_columns.contains(&"episode_image_checked_at".to_string()));

    let index_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'index' AND name = 'idx_media_files_episode_image_pending'",
            [],
            |row| row.get(0),
        )
        .expect("comprobar índice");
    assert_eq!(index_exists, 1);
}

#[test]
fn categorias_pliegan_tildes_y_rechazan_nombres_vacios() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    let mut repo = MediaRepository::new(&mut conn);
    let accion = repo
        .get_or_create_category("Acción")
        .expect("crear Acción")
        .expect("categoría válida");
    let accion_sin_tilde = repo
        .get_or_create_category("accion")
        .expect("crear accion")
        .expect("categoría válida");
    let accion_con_espacios = repo
        .get_or_create_category("  Acción  ")
        .expect("crear Acción con espacios")
        .expect("categoría válida");

    assert_eq!(accion, accion_sin_tilde);
    assert_eq!(accion, accion_con_espacios);
    assert_eq!(
        repo.get_or_create_category("   ").expect("nombre vacío"),
        None,
        "un nombre vacío no debe inventar una categoría"
    );

    let categories = repo.list_categories().expect("listar categorías");
    assert_eq!(categories.len(), 1);
    assert_eq!(categories[0].name, "Acción");
    assert_eq!(categories[0].slug, "accion");
}

#[test]
fn reindexar_resetea_la_imagen_de_episodio_si_cambia_la_posicion() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    let media_id = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.upsert(&MediaInput {
            r#type: "series".to_string(),
            title: "Show".to_string(),
            sort_title: "show".to_string(),
            year: None,
            folder_path: "/media/show".to_string(),
            parent_id: None,
        })
        .expect("upsert media")
        .id
    };

    let path = "/media/show/S01E01.mkv".to_string();
    let file = |season: i32, episode: i32| MediaFileInput {
        media_id,
        season: Some(season),
        episode: Some(episode),
        title: Some(format!("S{season:02}E{episode:02}")),
        file_path: path.clone(),
        size_bytes: None,
        mtime_ms: None,
    };

    {
        let mut repo = FileRepository::new(&mut conn);
        repo.upsert_many(&[file(1, 1)]).expect("insertar archivo");
        repo.set_episode_image(&path, "https://example.com/ep1x1.jpg")
            .expect("guardar imagen de episodio");
    }

    // Reindexar con la misma posición conserva la imagen ya descargada.
    {
        let mut repo = FileRepository::new(&mut conn);
        repo.upsert_many(&[file(1, 1)]).expect("reindexar igual");
        let files = repo.list_by_media(media_id).expect("listar archivos");
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].episode_image_path.as_deref(),
            Some("https://example.com/ep1x1.jpg")
        );
        assert!(files[0].episode_image_checked_at.is_some());
    }

    // Si cambia la temporada/episodio, la imagen anterior deja de ser válida.
    {
        let mut repo = FileRepository::new(&mut conn);
        repo.upsert_many(&[file(2, 1)])
            .expect("reindexar con cambio");
        let files = repo.list_by_media(media_id).expect("listar archivos");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].season, Some(2));
        assert_eq!(files[0].episode, Some(1));
        assert!(files[0].episode_image_path.is_none());
        assert!(files[0].episode_image_checked_at.is_none());
    }
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
            Some("https://example.com/banner.jpg"),
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
    assert_eq!(
        updated.banner_path.as_deref(),
        Some("https://example.com/banner.jpg")
    );
    assert_eq!(updated.synopsis.as_deref(), Some("Sinopsis de prueba"));
    assert!(updated.metadata_fetched_at.is_some());
}

#[test]
fn reindexar_conserva_metadatos_sincronizados() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    let media_id = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.upsert(&MediaInput {
            r#type: "series".to_string(),
            title: "nombre de carpeta".to_string(),
            sort_title: "nombre de carpeta".to_string(),
            year: None,
            folder_path: "/media/serie".to_string(),
            parent_id: None,
        })
        .expect("upsert inicial")
        .id
    };

    {
        let mut repo = MediaRepository::new(&mut conn);
        repo.overwrite_metadata(
            media_id,
            &MediaMetadataInput {
                title: "Título Sincronizado",
                sort_title: "titulo sincronizado",
                year: Some(2024),
                poster_path: Some("https://example.com/poster.jpg"),
                banner_path: Some("https://example.com/banner.jpg"),
                synopsis: Some("Sinopsis sincronizada"),
            },
        )
        .expect("sincronizar metadatos");
    }

    // Reindexar vuelve a llamar a `upsert` con los datos derivados de la carpeta.
    {
        let mut repo = MediaRepository::new(&mut conn);
        repo.upsert(&MediaInput {
            r#type: "series".to_string(),
            title: "nombre de carpeta".to_string(),
            sort_title: "nombre de carpeta".to_string(),
            year: None,
            folder_path: "/media/serie".to_string(),
            parent_id: None,
        })
        .expect("reindexar");
    }

    let media = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.find_by_id(media_id)
            .expect("buscar media")
            .expect("media existente")
    };

    assert_eq!(media.title, "Título Sincronizado");
    assert_eq!(media.sort_title, "titulo sincronizado");
    assert_eq!(media.year, Some(2024));
    assert_eq!(media.synopsis.as_deref(), Some("Sinopsis sincronizada"));
    assert_eq!(
        media.poster_path.as_deref(),
        Some("https://example.com/poster.jpg")
    );
    assert!(media.metadata_fetched_at.is_some());
}

#[test]
fn update_metadata_conserva_el_banner_si_no_se_recibe_uno_nuevo() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    let media = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.upsert(&MediaInput {
            r#type: "anime".to_string(),
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
            Some("https://example.com/banner.jpg"),
            Some("Sinopsis"),
        )
        .expect("guardar metadatos con banner");
        // Una sincronización posterior sin banner no debe borrarlo.
        repo.update_metadata(
            media.id,
            Some(2024),
            Some("https://example.com/poster.jpg"),
            None,
            None,
        )
        .expect("actualizar sin banner");
    }

    let updated = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.find_by_id(media.id)
            .expect("buscar media")
            .expect("media existente")
    };
    assert_eq!(
        updated.banner_path.as_deref(),
        Some("https://example.com/banner.jpg")
    );
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
            Some("https://example.com/old-banner.jpg"),
            Some("Sinopsis antigua"),
        )
        .expect("guardar metadatos antiguos");
        media.id
    };

    {
        let mut repo = MediaRepository::new(&mut conn);
        repo.overwrite_metadata(
            media_id,
            &MediaMetadataInput {
                title: "Esmeralda Nueva",
                sort_title: "esmeralda nueva",
                year: Some(2025),
                poster_path: Some("https://example.com/new.jpg"),
                banner_path: Some("https://example.com/new-banner.jpg"),
                synopsis: Some("Sinopsis nueva"),
            },
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
    assert_eq!(
        media.banner_path.as_deref(),
        Some("https://example.com/new-banner.jpg")
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

#[test]
fn categorias_crear_listar_y_asociar() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    let media_id = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.upsert(&MediaInput {
            r#type: "series".to_string(),
            title: "Solo Leveling".to_string(),
            sort_title: "solo leveling".to_string(),
            year: Some(2024),
            folder_path: "/media/solo-leveling".to_string(),
            parent_id: None,
        })
        .expect("upsert media")
        .id
    };

    // Crear categorías y asociarlas al medio.
    {
        let mut repo = MediaRepository::new(&mut conn);
        let accion = repo
            .get_or_create_category("Acción")
            .expect("crear categoría Acción")
            .expect("categoría válida");
        let animacion = repo
            .get_or_create_category("Animación")
            .expect("crear categoría Animación")
            .expect("categoría válida");
        repo.set_media_categories(media_id, &[accion, animacion])
            .expect("asociar categorías");
    }

    // Listar categorías disponibles.
    let todas = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.list_categories().expect("listar categorías")
    };
    assert_eq!(todas.len(), 2);
    assert!(todas.iter().any(|c| c.name == "Acción"));
    assert!(todas.iter().any(|c| c.name == "Animación"));

    // Cargar categorías del medio.
    let cats_media = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.load_categories_for_media(media_id)
            .expect("categorías del medio")
    };
    assert_eq!(cats_media.len(), 2);
    assert!(cats_media.contains(&"Acción".to_string()));
    assert!(cats_media.contains(&"Animación".to_string()));

    // El slug se normaliza (sin duplicados al volver a crear).
    {
        let mut repo = MediaRepository::new(&mut conn);
        let accion_otra = repo
            .get_or_create_category("Acción")
            .expect("recrear categoría Acción")
            .expect("categoría válida");
        assert_eq!(
            todas.iter().find(|c| c.name == "Acción").unwrap().id,
            accion_otra,
            "no debe duplicar la categoría existente"
        );
    }
}

#[test]
fn filtrado_por_categoria_devuelve_solo_medios_asociados() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    let (m1, m2) = {
        let mut repo = MediaRepository::new(&mut conn);
        let m1 = repo
            .upsert(&MediaInput {
                r#type: "series".to_string(),
                title: "Serie Acción".to_string(),
                sort_title: "serie accion".to_string(),
                year: Some(2024),
                folder_path: "/media/serie-accion".to_string(),
                parent_id: None,
            })
            .expect("upsert m1")
            .id;
        let m2 = repo
            .upsert(&MediaInput {
                r#type: "series".to_string(),
                title: "Serie Comedia".to_string(),
                sort_title: "serie comedia".to_string(),
                year: Some(2024),
                folder_path: "/media/serie-comedia".to_string(),
                parent_id: None,
            })
            .expect("upsert m2")
            .id;
        (m1, m2)
    };

    // Asociar categorías.
    {
        let mut repo = MediaRepository::new(&mut conn);
        let accion = repo
            .get_or_create_category("Acción")
            .expect("crear Acción")
            .expect("categoría válida");
        let comedia = repo
            .get_or_create_category("Comedia")
            .expect("crear Comedia")
            .expect("categoría válida");
        repo.set_media_categories(m1, &[accion])
            .expect("asociar Acción a m1");
        repo.set_media_categories(m2, &[comedia])
            .expect("asociar Comedia a m2");
    }

    // Filtrar por Acción: solo m1.
    let por_accion = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.list_by_category(
            1, // id de "Acción" (primera creada)
            ListMediaOptions {
                r#type: None,
                limit: Some(50),
                offset: Some(0),
                order_by: None,
            },
        )
        .expect("filtrar por categoría")
    };
    assert_eq!(por_accion.len(), 1);
    assert_eq!(por_accion[0].title, "Serie Acción");

    // Filtrar por Comedia: solo m2.
    let por_comedia = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.list_by_category(
            2, // id de "Comedia"
            ListMediaOptions {
                r#type: None,
                limit: Some(50),
                offset: Some(0),
                order_by: None,
            },
        )
        .expect("filtrar por categoría")
    };
    assert_eq!(por_comedia.len(), 1);
    assert_eq!(por_comedia[0].title, "Serie Comedia");
}

#[test]
fn vote_average_guardado_y_recuperado() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    let media_id = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.upsert(&MediaInput {
            r#type: "series".to_string(),
            title: "Solo Leveling".to_string(),
            sort_title: "solo leveling".to_string(),
            year: Some(2024),
            folder_path: "/media/solo-leveling".to_string(),
            parent_id: None,
        })
        .expect("upsert media")
        .id
    };

    {
        let mut repo = MediaRepository::new(&mut conn);
        repo.set_vote_average(media_id, Some(8.5))
            .expect("guardar vote_average");
    }

    let media = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.find_by_id(media_id)
            .expect("buscar media")
            .expect("media existente")
    };
    assert_eq!(media.vote_average, Some(8.5));

    // Sin puntuación: None.
    let media_id2 = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.upsert(&MediaInput {
            r#type: "movie".to_string(),
            title: "Película sin puntuación".to_string(),
            sort_title: "pelicula sin puntuacion".to_string(),
            year: None,
            folder_path: "/media/pelicula".to_string(),
            parent_id: None,
        })
        .expect("upsert media 2")
        .id
    };
    let media2 = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.find_by_id(media_id2)
            .expect("buscar media")
            .expect("media existente")
    };
    assert_eq!(media2.vote_average, None);
}

#[test]
fn episode_image_path_en_media_files() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    let media_id = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.upsert(&MediaInput {
            r#type: "series".to_string(),
            title: "Serie".to_string(),
            sort_title: "serie".to_string(),
            year: None,
            folder_path: "/media/serie".to_string(),
            parent_id: None,
        })
        .expect("upsert media")
        .id
    };

    // Insertar archivo con imagen de episodio.
    {
        // Upsert_many no soporta episode_image_path directamente; usamos SQL.
        conn.execute(
            "INSERT INTO media_files (media_id, season, episode, title, file_path, episode_image_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                media_id,
                1,
                1,
                "Episodio 1",
                "/media/serie/S01E01.mkv",
                "https://example.com/episode1.jpg",
            ],
        )
        .expect("insertar archivo con imagen de episodio");
    }

    let files = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.list_files_by_media(media_id).expect("listar archivos")
    };
    assert_eq!(files.len(), 1);
    assert_eq!(
        files[0].episode_image_path.as_deref(),
        Some("https://example.com/episode1.jpg")
    );
}

#[test]
fn historial_mantiene_una_entrada_activa_por_archivo() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    let media = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.upsert(&MediaInput {
            r#type: "series".to_string(),
            title: "Serie".to_string(),
            sort_title: "serie".to_string(),
            year: None,
            folder_path: "/media/serie".to_string(),
            parent_id: None,
        })
        .expect("insertar medio")
    };
    let file_path = "/media/serie/s01e01.mkv";

    {
        let mut repo = MediaRepository::new(&mut conn);
        repo.record_playback(media.id, file_path, 10.0, 100.0)
            .expect("primer guardado");
        repo.record_playback(media.id, file_path, 40.0, 100.0)
            .expect("segundo guardado");

        let probe = open_db(db_path.to_str().unwrap()).expect("abrir conexión de prueba");
        let active: i64 = probe
            .query_row(
                "SELECT COUNT(*) FROM playback_history
                 WHERE file_path = ?1 AND completed_at IS NULL",
                [file_path],
                |row| row.get(0),
            )
            .expect("contar entradas activas");
        assert_eq!(active, 1, "no debe duplicar la reproducción pendiente");

        let stored = repo
            .get_playback_position(file_path)
            .expect("leer posición");
        assert_eq!(stored, Some(40.0), "se conserva el progreso más avanzado");

        let history = repo
            .list_playback_history(Some(10), Some(0))
            .expect("listar historial");
        assert_eq!(history.len(), 1);

        // El índice único parcial también protege ante inserciones directas.
        let duplicate = probe.execute(
            "INSERT INTO playback_history (media_id, file_path, position, duration)
             VALUES (?1, ?2, 0, 0)",
            rusqlite::params![media.id, file_path],
        );
        assert!(duplicate.is_err(), "no debe admitir dos entradas activas");

        repo.complete_playback(media.id, file_path)
            .expect("completar reproducción");
        assert_eq!(
            repo.get_playback_position(file_path)
                .expect("leer tras completar"),
            None,
            "una reproducción completada no debe reanudarse"
        );
    }

    {
        let mut repo = MediaRepository::new(&mut conn);
        repo.record_playback(media.id, file_path, 0.0, 100.0)
            .expect("nueva reproducción");
        let probe = open_db(db_path.to_str().unwrap()).expect("abrir conexión de prueba");
        let total: i64 = probe
            .query_row(
                "SELECT COUNT(*) FROM playback_history WHERE file_path = ?1",
                [file_path],
                |row| row.get(0),
            )
            .expect("contar historial");
        assert_eq!(total, 2, "se conserva la entrada completada como historial");
        assert_eq!(
            repo.get_playback_position(file_path).expect("leer nueva"),
            Some(0.0)
        );

        let history = repo
            .list_playback_history(Some(10), Some(0))
            .expect("listar historial");
        assert_eq!(
            history.len(),
            1,
            "el historial muestra solo la entrada más reciente del archivo"
        );
    }
}

#[test]
fn migracion_archiva_duplicados_de_historial_sin_borrarlos() {
    let (db_path, _dir) = temp_db();

    {
        let conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");
        conn.execute(
            "INSERT INTO media (type, title, sort_title, folder_path)
             VALUES ('movie', 'Pelicula', 'pelicula', '/media/pelicula')",
            [],
        )
        .expect("insertar medio");

        // Simula una base de datos anterior a la migración 15 con tres
        // reproducciones pendientes del mismo archivo.
        conn.execute_batch(
            "DROP INDEX IF EXISTS idx_playback_history_active_file;
             DROP INDEX IF EXISTS idx_playback_history_file;
             ALTER TABLE playback_history DROP COLUMN completed_at;
             DELETE FROM schema_migrations WHERE version = 15;",
        )
        .expect("simular BD antigua");

        for position in [10.0, 20.0, 30.0] {
            conn.execute(
                "INSERT INTO playback_history (media_id, file_path, position, duration)
                 VALUES (1, '/media/pelicula/video.mkv', ?1, 100)",
                rusqlite::params![position],
            )
            .expect("insertar duplicado");
        }
    }

    let conn = open_db(db_path.to_str().unwrap()).expect("reabrir BD");
    let rows: Vec<(i64, Option<String>)> = conn
        .prepare(
            "SELECT id, completed_at FROM playback_history
             WHERE file_path = '/media/pelicula/video.mkv' ORDER BY id",
        )
        .expect("preparar consulta")
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let completed: Option<String> = row.get(1)?;
            Ok((id, completed))
        })
        .expect("leer historial")
        .map(Result::unwrap)
        .collect();

    assert_eq!(rows.len(), 3, "no se elimina el historial legítimo");
    assert!(rows[0].1.is_some(), "el duplicado antiguo queda archivado");
    assert!(rows[1].1.is_some(), "el duplicado antiguo queda archivado");
    assert!(
        rows[2].1.is_none(),
        "solo la entrada más reciente sigue activa"
    );

    let index_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'index' AND name = 'idx_playback_history_active_file'",
            [],
            |row| row.get(0),
        )
        .expect("comprobar índice");
    assert_eq!(index_exists, 1);
}

/// Test de integración con un archivo real de la biblioteca del usuario.
///
/// Verifica el flujo completo de selección de subtítulos:
/// 1. Indexar un archivo real (Black Torch S01E01 con 11 subtítulos embebidos).
/// 2. Guardar una preferencia de subtítulo (Spanish[ESP], stream_index=6).
/// 3. Leer la preferencia guardada.
/// 4. Resolver la preferencia contra los streams actuales.
/// 5. Generar los argumentos de mpv y verificar que --sid apunta al subtitle correcto.
#[test]
fn seleccion_de_subtitulos_con_archivo_real_de_biblioteca() {
    let video_path =
        "/home/coronel/Videos/downloads/Black Torch/[Judas] Black Torch - S01E01v2.mkv";
    assert!(
        std::path::Path::new(video_path).exists(),
        "el archivo real de la biblioteca debe existir: {video_path}"
    );

    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    // 1. Insertar el medio (serie) y el archivo de vídeo.
    let media_id = {
        let mut repo = MediaRepository::new(&mut conn);
        let media = repo
            .upsert(&MediaInput {
                r#type: "series".to_string(),
                title: "Black Torch".to_string(),
                sort_title: "black torch".to_string(),
                year: Some(2023),
                folder_path: "/home/coronel/Videos/downloads/Black Torch".to_string(),
                parent_id: None,
            })
            .expect("upsert media Black Torch");
        media.id
    };

    {
        let mut repo = FileRepository::new(&mut conn);
        repo.upsert_many(&[MediaFileInput {
            media_id,
            season: Some(1),
            episode: Some(1),
            title: Some("S01E01".to_string()),
            file_path: video_path.to_string(),
            size_bytes: None,
            mtime_ms: None,
        }])
        .expect("insertar media_file");
    }

    // 2. Extraer streams reales con ffprobe (igual que el indexador) e
    //    insertarlos en la BD (audio + subtítulo, índice relativo al tipo).
    let streams_reales = acryl::core::library::extract_media_streams(video_path)
        .expect("extraer streams reales con ffprobe");
    for stream in &streams_reales {
        conn.execute(
            "INSERT INTO media_streams
             (media_id, file_path, stream_type, stream_index, language, codec, title, external_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                media_id,
                video_path,
                stream.stream_type,
                stream.stream_index,
                stream.language,
                stream.codec,
                stream.title,
                stream.external_path,
            ],
        )
        .expect("insertar media_stream real");
    }

    // 3. Identificar audio japonés y subtítulo español por sus metadatos reales.
    let audio_japones = streams_reales
        .iter()
        .find(|s| s.stream_type == "audio" && s.language.as_deref() == Some("jpn"))
        .map(|s| s.clone().into_media_stream())
        .expect("debe haber audio japonés");
    let subtitulo_espanol = streams_reales
        .iter()
        .find(|s| {
            s.stream_type == "subtitle"
                && s.language.as_deref() == Some("spa")
                && s.title.as_deref() == Some("Spanish[ESP]")
        })
        .map(|s| s.clone().into_media_stream())
        .expect("debe haber subtítulo Spanish[ESP]");

    println!(
        "Audio japonés: stream_index={} key={}",
        audio_japones.stream_index,
        audio_japones.track_key()
    );
    println!(
        "Subtítulo español: stream_index={} key={}",
        subtitulo_espanol.stream_index,
        subtitulo_espanol.track_key()
    );
    // 3. Guardar la preferencia: audio japonés + subtítulo español.
    {
        let mut repo = FileRepository::new(&mut conn);
        repo.set_track_preference(
            video_path,
            &TrackPreference {
                selection: TrackSelection {
                    audio_stream_index: Some(audio_japones.stream_index),
                    subtitle_stream_index: Some(subtitulo_espanol.stream_index),
                    subtitles_disabled: false,
                },
                audio_track_key: Some(audio_japones.track_key()),
                subtitle_track_key: Some(subtitulo_espanol.track_key()),
                subtitle_external_path: None,
            },
        )
        .expect("guardar preferencia de audio y subtítulo");
    }

    // 4. Leer la preferencia guardada.
    let leída = {
        let mut repo = FileRepository::new(&mut conn);
        repo.track_preference(video_path).expect("leer preferencia")
    };
    assert_eq!(
        leída.selection.audio_stream_index,
        Some(audio_japones.stream_index)
    );
    assert_eq!(
        leída.selection.subtitle_stream_index,
        Some(subtitulo_espanol.stream_index)
    );
    assert_eq!(
        leída.audio_track_key.as_deref(),
        Some(audio_japones.track_key()).as_deref()
    );
    assert_eq!(
        leída.subtitle_track_key.as_deref(),
        Some(subtitulo_espanol.track_key()).as_deref()
    );
    assert!(!leída.selection.subtitles_disabled);

    // 5. Obtener los streams de la BD y resolver la preferencia.
    let streams = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.list_streams_by_file(video_path)
            .expect("listar streams")
    };
    assert_eq!(
        streams
            .iter()
            .filter(|s| s.stream_type == "subtitle")
            .count(),
        11,
        "deben haber 11 subtítulos embebidos"
    );

    let resuelto = resolve_track_preference(&leída, &streams);
    assert_eq!(
        resuelto.selection.audio_stream_index,
        Some(audio_japones.stream_index),
        "el audio resuelto debe ser el japonés"
    );
    assert_eq!(
        resuelto.selection.subtitle_stream_index,
        Some(subtitulo_espanol.stream_index),
        "el subtítulo resuelto debe ser el español"
    );
    assert!(!resuelto.selection.subtitles_disabled);

    // 6. Generar los argumentos de mpv y verificar --aid y --sid correctos.
    //    mpv numera desde 1 por tipo de pista.
    let args = build_mpv_launch_args_for_playback(video_path, &resuelto, PlayerWindowMode::Normal);
    assert!(
        args.contains(&format!("--aid={}", audio_japones.stream_index + 1)),
        "mpv debe recibir --aid={} para audio japonés, args: {args:?}",
        audio_japones.stream_index + 1
    );
    assert!(
        args.contains(&format!("--sid={}", subtitulo_espanol.stream_index + 1)),
        "mpv debe recibir --sid={} para subtítulo español, args: {args:?}",
        subtitulo_espanol.stream_index + 1
    );
    assert!(
        !args.iter().any(|a| a.starts_with("--sub-file=")),
        "no debe haber --sub-file (sin sidecars externos), args: {args:?}"
    );
    assert!(
        !args.iter().any(|a| a == "--sub-auto=no"),
        "no debe haber --sub-auto=no porque no hay sidecars externos, args: {args:?}"
    );

    // 7. Verificar que seleccionar Spanish[LAT] (stream_index=7) genera --sid=8.
    let preferencia_lat = TrackPreference {
        selection: TrackSelection {
            audio_stream_index: None,
            subtitle_stream_index: Some(7),
            subtitles_disabled: false,
        },
        ..TrackPreference::default()
    };
    let resuelto_lat = resolve_track_preference(&preferencia_lat, &streams);
    assert_eq!(resuelto_lat.selection.subtitle_stream_index, Some(7));
    let args_lat =
        build_mpv_launch_args_for_playback(video_path, &resuelto_lat, PlayerWindowMode::Normal);
    assert!(
        args_lat.contains(&"--sid=8".to_string()),
        "mpv debe recibir --sid=8 para Spanish[LAT], args: {args_lat:?}"
    );

    // 8. Verificar que "Ninguno" genera --sid=no.
    let preferencia_ninguno = TrackPreference {
        selection: TrackSelection {
            audio_stream_index: None,
            subtitle_stream_index: None,
            subtitles_disabled: true,
        },
        ..TrackPreference::default()
    };
    let resuelto_ninguno = resolve_track_preference(&preferencia_ninguno, &streams);
    assert!(resuelto_ninguno.selection.subtitles_disabled);
    let args_ninguno =
        build_mpv_launch_args_for_playback(video_path, &resuelto_ninguno, PlayerWindowMode::Normal);
    assert!(
        args_ninguno.contains(&"--sid=no".to_string()),
        "mpv debe recibir --sid=no para desactivar subtítulos, args: {args_ninguno:?}"
    );

    // 9. Verificar que "Automático" no pasa --sid (deja que mpv elija).
    let preferencia_auto = TrackPreference {
        selection: TrackSelection {
            audio_stream_index: None,
            subtitle_stream_index: None,
            subtitles_disabled: false,
        },
        ..TrackPreference::default()
    };
    let resuelto_auto = resolve_track_preference(&preferencia_auto, &streams);
    assert!(!resuelto_auto.selection.subtitles_disabled);
    assert_eq!(resuelto_auto.selection.subtitle_stream_index, None);
    let args_auto =
        build_mpv_launch_args_for_playback(video_path, &resuelto_auto, PlayerWindowMode::Normal);
    assert!(
        !args_auto.iter().any(|a| a.starts_with("--sid=")),
        "Automático no debe pasar --sid, args: {args_auto:?}"
    );
}

/// Test de integración end-to-end con un archivo real de la biblioteca del usuario.
///
/// Simula el flujo completo del usuario:
/// 1. Extraer streams reales con ffprobe (como hace el indexador).
/// 2. Guardar preferencia: audio japonés (stream_index=0) + subtítulo español (stream_index=6).
/// 3. Leer la preferencia guardada.
/// 4. Resolver la preferencia contra los streams actuales.
/// 5. Generar los argumentos de mpv y verificar que --aid y --sid apuntan a las pistas correctas.
#[test]
fn seleccion_de_audio_y_subtitulos_con_archivo_real_de_biblioteca() {
    let video_path =
        "/home/coronel/Videos/downloads/Black Torch/[Judas] Black Torch - S01E01v2.mkv";
    assert!(
        std::path::Path::new(video_path).exists(),
        "el archivo real de la biblioteca debe existir: {video_path}"
    );

    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    // 1. Insertar el medio y el archivo.
    let media_id = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.upsert(&MediaInput {
            r#type: "series".to_string(),
            title: "Black Torch".to_string(),
            sort_title: "black torch".to_string(),
            year: Some(2023),
            folder_path: "/home/coronel/Videos/downloads/Black Torch".to_string(),
            parent_id: None,
        })
        .expect("upsert media")
        .id
    };
    {
        let mut repo = FileRepository::new(&mut conn);
        repo.upsert_many(&[MediaFileInput {
            media_id,
            season: Some(1),
            episode: Some(1),
            title: Some("S01E01".to_string()),
            file_path: video_path.to_string(),
            size_bytes: None,
            mtime_ms: None,
        }])
        .expect("insertar media_file");
    }

    // Insertar los streams reales en la BD (como haría el indexador).
    let streams_reales = acryl::core::library::extract_media_streams(video_path)
        .expect("extraer streams reales con ffprobe");
    for stream in &streams_reales {
        conn.execute(
            "INSERT INTO media_streams
             (media_id, file_path, stream_type, stream_index, language, codec, title, external_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                media_id,
                video_path,
                stream.stream_type,
                stream.stream_index,
                stream.language,
                stream.codec,
                stream.title,
                stream.external_path,
            ],
        )
        .expect("insertar media_stream real");
    }

    // 2. Extraer streams reales con ffprobe (igual que el indexador).
    let streams = acryl::core::library::extract_media_streams(video_path)
        .expect("extraer streams con ffprobe");
    let audios: Vec<_> = streams
        .iter()
        .filter(|s| s.stream_type == "audio")
        .collect();
    let subtitles: Vec<_> = streams
        .iter()
        .filter(|s| s.stream_type == "subtitle")
        .collect();
    assert!(!audios.is_empty(), "debe haber al menos un audio");
    assert!(!subtitles.is_empty(), "debe haber al menos un subtítulo");

    // Identificar audio japonés y subtítulo español por sus metadatos reales.
    let audio_japones = streams
        .iter()
        .find(|s| s.stream_type == "audio" && s.language.as_deref() == Some("jpn"))
        .map(|s| s.clone().into_media_stream())
        .expect("debe haber audio japonés");
    let subtitulo_espanol = streams
        .iter()
        .find(|s| {
            s.stream_type == "subtitle"
                && s.language.as_deref() == Some("spa")
                && s.title.as_deref() == Some("Spanish[ESP]")
        })
        .map(|s| s.clone().into_media_stream())
        .expect("debe haber subtítulo Spanish[ESP]");

    println!(
        "Audio japonés: stream_index={} key={}",
        audio_japones.stream_index,
        audio_japones.track_key()
    );
    println!(
        "Subtítulo español: stream_index={} key={}",
        subtitulo_espanol.stream_index,
        subtitulo_espanol.track_key()
    );

    // 3. Guardar preferencia: audio japonés + subtítulo español.
    {
        let mut repo = FileRepository::new(&mut conn);
        repo.set_track_preference(
            video_path,
            &TrackPreference {
                selection: TrackSelection {
                    audio_stream_index: Some(audio_japones.stream_index),
                    subtitle_stream_index: Some(subtitulo_espanol.stream_index),
                    subtitles_disabled: false,
                },
                audio_track_key: Some(audio_japones.track_key()),
                subtitle_track_key: Some(subtitulo_espanol.track_key()),
                subtitle_external_path: None,
            },
        )
        .expect("guardar preferencia");
    }

    // 4. Leer la preferencia guardada.
    let leída = {
        let mut repo = FileRepository::new(&mut conn);
        repo.track_preference(video_path).expect("leer preferencia")
    };
    assert_eq!(
        leída.selection.audio_stream_index,
        Some(audio_japones.stream_index)
    );
    assert_eq!(
        leída.selection.subtitle_stream_index,
        Some(subtitulo_espanol.stream_index)
    );
    assert_eq!(
        leída.audio_track_key.as_deref(),
        Some(audio_japones.track_key()).as_deref()
    );
    assert_eq!(
        leída.subtitle_track_key.as_deref(),
        Some(subtitulo_espanol.track_key()).as_deref()
    );
    assert!(!leída.selection.subtitles_disabled);

    // 5. Resolver contra los streams actuales (como hace saved_tracks al reproducir).
    let streams_desde_bd = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.list_streams_by_file(video_path)
            .expect("listar streams")
    };
    assert_eq!(
        streams_desde_bd
            .iter()
            .filter(|s| s.stream_type == "audio")
            .count(),
        audios.len()
    );
    assert_eq!(
        streams_desde_bd
            .iter()
            .filter(|s| s.stream_type == "subtitle")
            .count(),
        subtitles.len()
    );

    let resuelto = resolve_track_preference(&leída, &streams_desde_bd);
    assert_eq!(
        resuelto.selection.audio_stream_index,
        Some(audio_japones.stream_index),
        "el audio resuelto debe ser el japonés"
    );
    assert_eq!(
        resuelto.selection.subtitle_stream_index,
        Some(subtitulo_espanol.stream_index),
        "el subtítulo resuelto debe ser el español"
    );
    assert!(!resuelto.selection.subtitles_disabled);

    // 6. Generar args de mpv y verificar --aid y --sid correctos.
    //    mpv numera desde 1: audio stream_index=0 -> --aid=1, subtitle stream_index=6 -> --sid=7.
    let args = build_mpv_launch_args_for_playback(video_path, &resuelto, PlayerWindowMode::Normal);
    assert!(
        args.contains(&format!("--aid={}", audio_japones.stream_index + 1)),
        "mpv debe recibir --aid={} para audio japonés, args: {args:?}",
        audio_japones.stream_index + 1
    );
    assert!(
        args.contains(&format!("--sid={}", subtitulo_espanol.stream_index + 1)),
        "mpv debe recibir --sid={} para subtítulo español, args: {args:?}",
        subtitulo_espanol.stream_index + 1
    );
    assert!(
        !args.iter().any(|a| a.starts_with("--sub-file=")),
        "no debe haber --sub-file (sin sidecars externos), args: {args:?}"
    );
}
