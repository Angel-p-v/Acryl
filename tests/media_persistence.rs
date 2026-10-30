//! Tests de integración para la persistencia de medios.
//! Ejercitan migraciones, repositorios y servicios de dominio con BD SQLite temporal.

use acryl::db::connection::open_db;
use acryl::db::file_repo::{FileRepository, MediaFileInput};
use acryl::db::media_repo::{ListMediaOptions, MediaInput, MediaRepository};
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
