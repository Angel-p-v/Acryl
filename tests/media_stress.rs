//! Tests de estrés y rendimiento con colecciones grandes.
//!
//! Verifica que la aplicación no bloquee la UI ni tenga problemas de
//! rendimiento al indexar miles de medios.

use acryl::db::connection::open_db;
use acryl::db::media_repo::{ListMediaOptions, MediaInput, MediaRepository, SearchMediaOptions};
use rusqlite::Connection;
use std::path::PathBuf;
use std::time::Instant;
use tempfile::TempDir;

/// Crea una BD temporal vacía y devuelve la ruta y el directorio temporal.
fn temp_db() -> (PathBuf, TempDir) {
    let dir = TempDir::new().expect("crear directorio temporal");
    let path = dir.path().join("test.db");
    (path, dir)
}

/// Inserta `n` medios de prueba en la BD y devuelve el tiempo transcurrido.
fn insertar_medios(conn: &mut Connection, n: usize) -> std::time::Duration {
    let inicio = Instant::now();
    let mut repo = MediaRepository::new(conn);
    for i in 0..n {
        let _ = repo.upsert(&MediaInput {
            r#type: if i % 3 == 0 { "movie" } else { "series" }.to_string(),
            title: format!("Título de prueba {}", i),
            sort_title: format!("titulo de prueba {}", i),
            year: Some(2000 + (i as i32 % 25)),
            folder_path: format!("/media/prueba-{}", i),
            parent_id: None,
        });
    }
    inicio.elapsed()
}

#[test]
fn insercion_de_miles_de_medios_es_rapida() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    const N: usize = 5000;
    let duracion = insertar_medios(&mut conn, N);

    // La inserción de 5000 medios no debería tardar más de 10 segundos
    // en hardware modesto (SQLite en WAL mode es rápido).
    assert!(
        duracion.as_secs() < 10,
        "insertar {} medios tomó {:?}, demasiado lento",
        N,
        duracion
    );

    // Verificar que todos los medios se insertaron.
    let mut repo = MediaRepository::new(&mut conn);
    let todos = repo
        .list(ListMediaOptions {
            r#type: None,
            limit: Some(N as i64 + 10),
            offset: Some(0),
            order_by: None,
        })
        .expect("listar todos");
    assert_eq!(todos.len(), N);
}

#[test]
fn consulta_paginada_es_rapida_con_miles_de_medios() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    insertar_medios(&mut conn, 5000);

    let mut repo = MediaRepository::new(&mut conn);
    let inicio = Instant::now();
    let pagina = repo
        .list(ListMediaOptions {
            r#type: None,
            limit: Some(100),
            offset: Some(0),
            order_by: None,
        })
        .expect("listar página");
    let duracion = inicio.elapsed();

    assert_eq!(pagina.len(), 100);
    assert!(
        duracion.as_millis() < 100,
        "consulta paginada de 100 ítems tomó {:?}, demasiado lento",
        duracion
    );
}

#[test]
fn consulta_paginada_por_tipo_es_rapida() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    insertar_medios(&mut conn, 5000);

    let mut repo = MediaRepository::new(&mut conn);
    let inicio = Instant::now();
    let peliculas = repo
        .list(ListMediaOptions {
            r#type: Some("movie".to_string()),
            limit: Some(100),
            offset: Some(0),
            order_by: None,
        })
        .expect("listar películas");
    let duracion = inicio.elapsed();

    // Aproximadamente 1/3 de los medios son películas.
    assert!(!peliculas.is_empty());
    assert!(
        duracion.as_millis() < 100,
        "consulta filtrada por tipo tomó {:?}, demasiado lento",
        duracion
    );
}

#[test]
fn busqueda_fts_con_miles_de_medios_es_rapida() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    insertar_medios(&mut conn, 5000);

    let mut repo = MediaRepository::new(&mut conn);
    let inicio = Instant::now();
    let resultados = repo
        .search(
            "prueba",
            SearchMediaOptions {
                r#type: None,
                limit: Some(50),
            },
        )
        .expect("buscar");
    let duracion = inicio.elapsed();

    // Debería encontrar muchos resultados (todos contienen "prueba" en el título).
    assert!(
        !resultados.is_empty(),
        "búsqueda FTS debería encontrar resultados"
    );
    assert!(
        duracion.as_millis() < 200,
        "búsqueda FTS con 5000 medios tomó {:?}, demasiado lento",
        duracion
    );
}

#[test]
fn paginacion_devuelve_medios_diferentes_en_cada_pagina() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    insertar_medios(&mut conn, 500);

    let mut repo = MediaRepository::new(&mut conn);
    let pagina1 = repo
        .list(ListMediaOptions {
            r#type: None,
            limit: Some(50),
            offset: Some(0),
            order_by: None,
        })
        .expect("listar página 1");
    let pagina2 = repo
        .list(ListMediaOptions {
            r#type: None,
            limit: Some(50),
            offset: Some(50),
            order_by: None,
        })
        .expect("listar página 2");

    assert_eq!(pagina1.len(), 50);
    assert_eq!(pagina2.len(), 50);

    // Las páginas no deberían superponerse.
    let ids1: std::collections::HashSet<i64> = pagina1.iter().map(|m| m.id).collect();
    let ids2: std::collections::HashSet<i64> = pagina2.iter().map(|m| m.id).collect();
    let interseccion: Vec<_> = ids1.intersection(&ids2).collect();
    assert!(
        interseccion.is_empty(),
        "las páginas paginadas no deberían superponerse"
    );
}

#[test]
fn historial_de_reproduccion_con_miles_de_registros_es_rapido() {
    let (db_path, _dir) = temp_db();
    let mut conn = open_db(db_path.to_str().unwrap()).expect("abrir BD");

    // Insertar un medio y miles de reproducciones.
    let media_id = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.upsert(&MediaInput {
            r#type: "movie".to_string(),
            title: "Película de prueba".to_string(),
            sort_title: "pelicula de prueba".to_string(),
            year: Some(2024),
            folder_path: "/media/prueba".to_string(),
            parent_id: None,
        })
        .expect("upsert")
        .id
    };

    let inicio = Instant::now();
    for i in 0..5000 {
        let _ = conn.execute(
            "INSERT INTO playback_history (media_id, file_path, position, duration)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                media_id,
                format!("/media/prueba/file_{}.mp4", i),
                0.0,
                3600.0
            ],
        );
    }
    let duracion_insercion = inicio.elapsed();
    assert!(
        duracion_insercion.as_secs() < 10,
        "insertar 5000 reproducciones tomó {:?}, demasiado lento",
        duracion_insercion
    );

    // Consultar el historial debería ser rápido gracias al índice.
    let mut repo = MediaRepository::new(&mut conn);
    let inicio = Instant::now();
    let historial = repo
        .list_playback_history(Some(100), Some(0))
        .expect("listar historial");
    let duracion_consulta = inicio.elapsed();

    assert_eq!(historial.len(), 100);
    assert!(
        duracion_consulta.as_millis() < 100,
        "consulta de historial con 5000 registros tomó {:?}, demasiado lento",
        duracion_consulta
    );
}
