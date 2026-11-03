//! Servicio de reproductor: delega la reproducción a `mpv` (proceso externo).
//!
//! La reproducción de vídeo se delega al reproductor `mpv` lanzado como
//! proceso externo mediante `std::process::Command`. El control (play, pausa,
//! stop, seek, volumen) se envía a `mpv` a través de su socket IPC.

use crate::db::file_repo::{FileRepository, TrackPreference, TrackSelection};
use crate::db::{MediaRepository, close_db, open_db};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

/// Identificador incremental para que dos reproductores (o dos tests en
/// paralelo) no compartan el mismo socket IPC.
static NEXT_SOCKET_ID: AtomicU64 = AtomicU64::new(0);

/// Directorio donde se crean los sockets IPC de mpv.
const MPV_SOCKET_DIR: &str = "/tmp";

/// Comando de reproductor externo por defecto.
const MPV_COMMAND: &str = "mpv";

/// Tiempo máximo de espera (ms) para que mpv cree su socket IPC.
const MPV_SOCKET_TIMEOUT_MS: u64 = 5000;

/// Modo de ventana que debe usar mpv al abrirse.
///
/// Se deriva del estado real de la ventana de Acryl para no forzar pantalla
/// completa cuando el usuario solo tenía la ventana maximizada. Confundir
/// ambos estados hacía que Hyprland reubicara Acryl al cerrar mpv.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayerWindowMode {
    #[default]
    Normal,
    Maximized,
    Fullscreen,
}

/// Pistas resueltas para un archivo concreto.
///
/// `external_subtitles` contiene las rutas de los sidecars en el mismo orden
/// que sus índices de subtítulo. Se pasan a mpv con `--sub-file` para que la
/// pista externa elegida tenga un `--sid` determinista.
#[derive(Debug, Clone, Default)]
pub struct ResolvedTracks {
    pub selection: TrackSelection,
    pub external_subtitles: Vec<String>,
}

/// Construye los argumentos de línea de comandos de mpv (puro, testeable).
pub fn build_mpv_args(file_path: &str, socket_path: &str) -> Vec<String> {
    build_mpv_args_with_tracks(file_path, socket_path, TrackSelection::default())
}

/// Construye los argumentos de mpv para el reproductor con IPC, aplicando la
/// selección de pistas. Los índices guardados son 0-based, mientras que mpv
/// numera las pistas de audio y subtítulos por separado empezando en 1.
pub fn build_mpv_args_with_tracks(
    file_path: &str,
    socket_path: &str,
    selection: TrackSelection,
) -> Vec<String> {
    build_mpv_args_for_playback(
        file_path,
        socket_path,
        &ResolvedTracks {
            selection,
            external_subtitles: Vec::new(),
        },
        PlayerWindowMode::Fullscreen,
    )
}

/// Construye los argumentos de mpv con IPC para un modo de ventana y pistas
/// resueltas concretas.
pub fn build_mpv_args_for_playback(
    file_path: &str,
    socket_path: &str,
    tracks: &ResolvedTracks,
    window_mode: PlayerWindowMode,
) -> Vec<String> {
    let mut args = vec![
        "--force-window=immediate".to_string(),
        format!("--input-ipc-server={}", socket_path),
        "--idle=yes".to_string(),
    ];
    append_window_args(&mut args, window_mode);
    append_track_args(&mut args, tracks);
    args.push(file_path.to_string());
    args
}

/// Construye los argumentos para abrir mpv como reproductor independiente.
pub fn build_mpv_launch_args(file_path: &str) -> Vec<String> {
    build_mpv_launch_args_with_tracks(file_path, TrackSelection::default())
}

/// Construye los argumentos del reproductor independiente con selección de
/// pistas. Las opciones deben ir antes de `--` para que mpv no las confunda
/// con rutas de archivo.
pub fn build_mpv_launch_args_with_tracks(
    file_path: &str,
    selection: TrackSelection,
) -> Vec<String> {
    build_mpv_launch_args_for_playback(
        file_path,
        &ResolvedTracks {
            selection,
            external_subtitles: Vec::new(),
        },
        PlayerWindowMode::Fullscreen,
    )
}

/// Construye los argumentos del reproductor independiente para un modo de
/// ventana y pistas resueltas concretas.
pub fn build_mpv_launch_args_for_playback(
    file_path: &str,
    tracks: &ResolvedTracks,
    window_mode: PlayerWindowMode,
) -> Vec<String> {
    let mut args = vec!["--force-window=immediate".to_string()];
    append_window_args(&mut args, window_mode);
    append_track_args(&mut args, tracks);
    args.push("--".to_string());
    args.push(file_path.to_string());
    args
}

/// Añade solo la opción de ventana solicitada. `normal` no añade nada para
/// respetar el tamaño que mpv tenga configurado por el usuario.
fn append_window_args(args: &mut Vec<String>, window_mode: PlayerWindowMode) {
    match window_mode {
        PlayerWindowMode::Normal => {}
        PlayerWindowMode::Maximized => args.push("--window-maximized=yes".to_string()),
        PlayerWindowMode::Fullscreen => args.push("--fullscreen".to_string()),
    }
}

/// Añade `--aid` y `--sid` según la selección. `subtitles_disabled` tiene
/// prioridad sobre una pista concreta para que «Ninguno» siempre desactive
/// los subtítulos.
///
/// Cuando hay subtítulos externos se desactiva la autocarga de mpv y se pasan
/// todos los sidecars en orden. Así el `--sid` elegido apunta siempre a la
/// pista correcta, sin depender de cómo ordene mpv los archivos descubiertos.
fn append_track_args(args: &mut Vec<String>, tracks: &ResolvedTracks) {
    let selection = tracks.selection;
    if let Some(audio_index) = selection.audio_stream_index {
        args.push(format!("--aid={}", audio_index + 1));
    }

    if selection.subtitles_disabled {
        args.push("--sid=no".to_string());
    } else if let Some(subtitle_index) = selection.subtitle_stream_index {
        args.push(format!("--sid={}", subtitle_index + 1));
    }

    let has_selected_subtitle = !selection.subtitles_disabled
        && (selection.subtitle_stream_index.is_some() || !tracks.external_subtitles.is_empty());
    if has_selected_subtitle && !tracks.external_subtitles.is_empty() {
        args.push("--sub-auto=no".to_string());
        for path in &tracks.external_subtitles {
            args.push(format!("--sub-file={}", path));
        }
    }
}

/// Señal de finalización de una reproducción lanzada como proceso externo.
///
/// La UI la usa para restaurar el estado de su ventana cuando mpv termina,
/// sin bloquear el hilo de GTK ni hacer polling del proceso.
pub struct PlaybackSession {
    finished: Receiver<()>,
}

impl PlaybackSession {
    /// Indica si mpv ya terminó. Pensado para consultarse desde un timeout de
    /// GLib; devuelve `true` también si el hilo terminó sin enviar la señal.
    pub fn finished(&self) -> bool {
        match self.finished.try_recv() {
            Ok(()) | Err(mpsc::TryRecvError::Disconnected) => true,
            Err(mpsc::TryRecvError::Empty) => false,
        }
    }
}

/// Valida la ruta y lanza el proceso mpv sin esperar a que termine.
fn spawn_mpv(args: &[String]) -> Result<Child, Box<dyn std::error::Error>> {
    Command::new(MPV_COMMAND)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| format!("no se pudo lanzar '{}': {}", MPV_COMMAND, e).into())
}

fn validate_media_path(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let is_url = file_path.starts_with("http://") || file_path.starts_with("https://");
    if !is_url && file_path != "-" && !std::path::Path::new(file_path).exists() {
        return Err(format!("archivo no encontrado: {}", file_path).into());
    }
    Ok(())
}

/// Indica si una reproducción terminada debe marcarse como completada.
///
/// Se conserva el umbral existente (5 minutos para episodios, 10 para
/// películas), pero también se completa cuando la posición alcanza o supera la
/// duración: antes esos casos dejaban una entrada activa al final del vídeo y
/// la siguiente reproducción empezaba en el último segundo.
fn should_complete_playback(media_type: &str, position: f64, duration: f64) -> bool {
    if duration <= 0.0 {
        return false;
    }
    let threshold = if media_type == "movie" { 600.0 } else { 300.0 };
    duration - position < threshold
}

/// Lanza mpv en su propia ventana para reproducir un archivo.
///
/// El proceso se espera en un hilo secundario para evitar dejar procesos
/// zombie cuando mpv termina.
pub fn launch_mpv(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    launch_mpv_with_tracks(file_path, TrackSelection::default())
}

/// Lanza mpv en su propia ventana aplicando la selección de pistas indicada.
pub fn launch_mpv_with_tracks(
    file_path: &str,
    selection: TrackSelection,
) -> Result<(), Box<dyn std::error::Error>> {
    validate_media_path(file_path)?;
    let mut child = spawn_mpv(&build_mpv_launch_args_with_tracks(file_path, selection))?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// Lee la preferencia guardada y la resuelve contra los streams actuales del
/// archivo. Un fallo de la base de datos no impide la reproducción: se usa el
/// comportamiento automático de mpv.
fn saved_tracks(db_path: &str, file_path: &str) -> ResolvedTracks {
    let Ok(mut conn) = open_db(db_path) else {
        return ResolvedTracks::default();
    };
    let preference = FileRepository::new(&mut conn)
        .track_preference(file_path)
        .unwrap_or_default();
    let mut streams = MediaRepository::new(&mut conn)
        .list_streams_by_file(file_path)
        .unwrap_or_default();
    close_db(conn);

    // Una base de datos indexada antes de soportar sidecars puede no tenerlos
    // en `media_streams`. Se descubren al reproducir para no perder la pista
    // externa guardada ni obligar a reindexar toda la biblioteca.
    let next_index = streams
        .iter()
        .filter(|stream| stream.stream_type == "subtitle")
        .map(|stream| stream.stream_index + 1)
        .max()
        .unwrap_or(0);
    for subtitle in crate::core::library::discover_external_subtitles(file_path, next_index) {
        if !streams
            .iter()
            .any(|stream| stream.external_path == subtitle.external_path)
        {
            streams.push(subtitle.into_media_stream());
        }
    }

    resolve_track_preference(&preference, &streams)
}

/// Resuelve una preferencia guardada contra los streams actuales.
///
/// La clave estable manda sobre el índice. Si la pista ya no existe, no se
/// reutiliza otra pista parecida: se desactivan los subtítulos. El índice solo
/// actúa como respaldo para bases de datos antiguas sin clave.
pub fn resolve_track_preference(
    preference: &TrackPreference,
    streams: &[crate::db::media_repo::MediaStream],
) -> ResolvedTracks {
    let audio_streams: Vec<_> = streams
        .iter()
        .filter(|stream| stream.stream_type == "audio")
        .collect();
    let subtitle_streams: Vec<_> = streams
        .iter()
        .filter(|stream| stream.stream_type == "subtitle")
        .collect();

    let audio_stream_index = match preference.audio_track_key.as_deref() {
        Some(key) => audio_streams
            .iter()
            .find(|stream| stream.track_key() == key)
            .map(|stream| stream.stream_index),
        None => preference.selection.audio_stream_index.filter(|index| {
            audio_streams
                .iter()
                .any(|stream| stream.stream_index == *index)
        }),
    };

    let mut external_subtitles: Vec<String> = subtitle_streams
        .iter()
        .filter_map(|stream| stream.external_path.clone())
        .collect();
    external_subtitles.sort();
    external_subtitles.dedup();

    if preference.selection.subtitles_disabled {
        return ResolvedTracks {
            selection: TrackSelection {
                audio_stream_index,
                subtitle_stream_index: None,
                subtitles_disabled: true,
            },
            external_subtitles,
        };
    }

    let resolved_subtitle: Option<(Option<i32>, Option<String>)> =
        match preference.subtitle_track_key.as_deref() {
            Some(key) => subtitle_streams
                .iter()
                .find(|stream| stream.track_key() == key)
                .map(|stream| (Some(stream.stream_index), stream.external_path.clone())),
            None => preference
                .selection
                .subtitle_stream_index
                .filter(|index| {
                    subtitle_streams
                        .iter()
                        .any(|stream| stream.stream_index == *index)
                })
                .map(|index| (Some(index), None)),
        };

    // Si había una preferencia de subtítulos pero la pista ya no existe, se
    // desactivan para no mostrar accidentalmente otra pista del mismo archivo.
    let subtitle_was_requested = preference.subtitle_track_key.is_some()
        || preference.selection.subtitle_stream_index.is_some();
    let (subtitle_stream_index, selected_external) = resolved_subtitle.unwrap_or((None, None));

    if let Some(path) = selected_external
        && !external_subtitles.iter().any(|known| known == &path)
    {
        external_subtitles.push(path);
        external_subtitles.sort();
    }

    ResolvedTracks {
        selection: TrackSelection {
            audio_stream_index,
            subtitle_stream_index,
            subtitles_disabled: subtitle_was_requested && subtitle_stream_index.is_none(),
        },
        external_subtitles,
    }
}

/// Lanza mpv y registra la reproducción en el historial si se conoce el medio.
pub fn play_external(
    db_path: &str,
    media_id: Option<i64>,
    file_path: &str,
) -> Result<PlaybackSession, Box<dyn std::error::Error>> {
    play_external_with_mode(db_path, media_id, file_path, PlayerWindowMode::Fullscreen)
}

/// Variante de `play_external` que respeta el estado de ventana de Acryl.
pub fn play_external_with_mode(
    db_path: &str,
    media_id: Option<i64>,
    file_path: &str,
    window_mode: PlayerWindowMode,
) -> Result<PlaybackSession, Box<dyn std::error::Error>> {
    validate_media_path(file_path)?;
    let tracks = saved_tracks(db_path, file_path);
    let mut child = spawn_mpv(&build_mpv_launch_args_for_playback(
        file_path,
        &tracks,
        window_mode,
    ))?;

    let (finished_sender, finished_receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = child.wait();
        let _ = finished_sender.send(());
    });

    if let Some(media_id) = media_id
        && let Err(error) = PlayerService::open(db_path, media_id, file_path)
    {
        eprintln!("No se pudo registrar la reproducción: {}", error);
    }

    Ok(PlaybackSession {
        finished: finished_receiver,
    })
}

/// Lanza mpv con soporte de reanudación desde una posición guardada y
/// marca automáticamente el archivo como visto al terminar si faltan
/// pocos minutos (5 min para episodios, 10 min para películas).
///
/// - Lee la última posición guardada en `playback_history` y la pasa a
///   mpv con `--start` para reanudar donde se quedó.
///   Hace polling de la posición cada segundo vía socket IPC para conocer
///   el progreso real.
/// - Respeta el modo de ventana solicitado (`normal`, `maximizado` o
///   `pantalla completa`) en lugar de forzar siempre `--fullscreen`.
///   Cuando mpv termina, si faltan menos de 5 min (episodio/anime) o
///   10 min (película) para terminar, marca `is_watched = true` en
///   `media_files`.
pub fn play_with_resume(
    db_path: &str,
    media_id: i64,
    file_path: &str,
) -> Result<PlaybackSession, Box<dyn std::error::Error>> {
    play_with_resume_with_mode(db_path, media_id, file_path, PlayerWindowMode::Fullscreen)
}

/// Variante de `play_with_resume` que usa el modo de ventana solicitado por
/// la UI (normal, maximizada o pantalla completa real).
pub fn play_with_resume_with_mode(
    db_path: &str,
    media_id: i64,
    file_path: &str,
    window_mode: PlayerWindowMode,
) -> Result<PlaybackSession, Box<dyn std::error::Error>> {
    validate_media_path(file_path)?;

    // 1. Consultar tipo de medio y última posición guardada (rápido, en hilo principal).
    let (media_type, start_position) = {
        let mut conn = open_db(db_path)?;
        let mut repo = MediaRepository::new(&mut conn);
        let t = repo.get_media_type(media_id)?.unwrap_or_default();
        let pos = repo.get_playback_position(file_path)?.unwrap_or(0.0);
        close_db(conn);
        (t, pos)
    };

    // 2. Preferencia de pistas guardada, resuelta contra los streams actuales.
    let tracks = saved_tracks(db_path, file_path);

    // 3. Crear socket IPC único para mpv.
    let socket_path = format!(
        "/tmp/acryl-mpv-{}.socket",
        NEXT_SOCKET_ID.fetch_add(1, Ordering::Relaxed)
    );

    // 4. Construir argumentos de mpv con IPC, start-position, pistas y modo
    //    de ventana. `--start` reanuda sin dejar que mpv reabra la ventana
    //    con un tamaño distinto al solicitado.
    let mut args = build_mpv_args_for_playback(file_path, &socket_path, &tracks, window_mode);
    // Insertar `--start` antes de la ruta del archivo.
    let insert_at = args.len().saturating_sub(1);
    args.insert(insert_at, format!("--start={}", start_position));

    // 5. Lanzar mpv.
    let mut child = spawn_mpv(&args)?;

    // 6. Registrar inicio de reproducción en historial. El repositorio hace
    //    UPSERT de la entrada activa para no duplicar reproducciones pendientes.
    {
        let mut conn = open_db(db_path)?;
        let mut repo = MediaRepository::new(&mut conn);
        let _ = repo.record_playback(media_id, file_path, start_position, 0.0);
        close_db(conn);
    }

    // 7. Hilo separado para monitorear mpv: hace polling de posición,
    //    espera a que termine, actualiza historial y marca como visto si
    //    faltan pocos minutos. Todo esto se hace fuera del hilo de GTK
    //    para no bloquear la interfaz.
    let db_path_owned = db_path.to_string();
    let file_path_owned = file_path.to_string();
    let media_type_owned = media_type;
    let socket_owned = socket_path.clone();
    let (finished_sender, finished_receiver) = mpsc::channel();

    std::thread::spawn(move || {
        let mut last_position = start_position;
        let mut last_duration = 0.0;

        // Darle tiempo a mpv para crear el socket IPC antes del primer intento.
        std::thread::sleep(Duration::from_millis(300));

        // Polling de posición y duración cada segundo hasta que mpv cierre el socket.
        // Reintenta la conexión varias veces al inicio (race condition con la
        // creación del socket) y deja de intentar cuando mpv ya no responde.
        let mut connect_failures = 0usize;
        loop {
            // Si mpv ya terminó, salir del bucle.
            if child.try_wait().is_ok_and(|s| s.is_some()) {
                break;
            }

            match UnixStream::connect(&socket_owned) {
                Ok(mut stream) => {
                    connect_failures = 0;
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));

                    // Leer time-pos
                    let msg = r#"{"command":["get_property","time-pos"]}"#;
                    let _ = stream.write_all(format!("{}\n", msg).as_bytes());
                    let mut reader = BufReader::new(&stream);
                    let mut response = String::new();
                    if reader.read_line(&mut response).is_ok()
                        && let Ok(v) = serde_json::from_str::<Value>(&response)
                        && let Some(pos) = v.get("data").and_then(|d| d.as_f64())
                    {
                        last_position = pos;
                    }
                    drop(stream);
                }
                Err(_) => {
                    connect_failures += 1;
                    // Si fallamos muchas veces seguidas y mpv sigue vivo,
                    // el socket puede no haberse creado aún; reintentar.
                    if connect_failures > 5 {
                        break;
                    }
                }
            }

            // Leer duration (nueva conexión para evitar bloqueos).
            match UnixStream::connect(&socket_owned) {
                Ok(mut stream) => {
                    connect_failures = 0;
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
                    let msg = r#"{"command":["get_property","duration"]}"#;
                    let _ = stream.write_all(format!("{}\n", msg).as_bytes());
                    let mut reader = BufReader::new(&stream);
                    let mut response = String::new();
                    if reader.read_line(&mut response).is_ok()
                        && let Ok(v) = serde_json::from_str::<Value>(&response)
                        && let Some(dur) = v.get("data").and_then(|d| d.as_f64())
                    {
                        last_duration = dur;
                    }
                    drop(stream);
                }
                Err(_) => {
                    connect_failures += 1;
                    if connect_failures > 5 {
                        break;
                    }
                }
            }

            std::thread::sleep(Duration::from_secs(1));
        }

        // Esperar a que el proceso mpv termine realmente.
        let _ = child.wait();

        // Actualizar el registro de reproducción con la posición final.
        if let Ok(mut conn) = open_db(&db_path_owned) {
            let mut repo = MediaRepository::new(&mut conn);
            let _ = repo.update_playback_position(
                media_id,
                &file_path_owned,
                last_position,
                last_duration,
            );
            close_db(conn);
        }

        // Marcar como visto si faltan menos del umbral o si el vídeo terminó.
        if should_complete_playback(&media_type_owned, last_position, last_duration)
            && let Ok(mut conn) = open_db(&db_path_owned)
        {
            let mut repo = MediaRepository::new(&mut conn);
            let _ = repo.set_file_watched(&file_path_owned, true);
            let _ = repo.complete_playback(media_id, &file_path_owned);
            close_db(conn);
        }

        // Limpiar socket IPC.
        let _ = std::fs::remove_file(&socket_owned);

        // Avisar a la UI de que puede restaurar el estado de su ventana.
        let _ = finished_sender.send(());
    });

    Ok(PlaybackSession {
        finished: finished_receiver,
    })
}

/// Ejecuta un comando JSON contra el socket IPC de mpv y devuelve la primera
/// línea de respuesta (si la hay).
fn mpv_request(
    socket_path: &str,
    command_json: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut stream = UnixStream::connect(socket_path).map_err(|e| {
        format!(
            "no se pudo conectar al socket IPC de mpv ({}): {}",
            socket_path, e
        )
    })?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;

    let msg = format!("{}\n", command_json);
    stream.write_all(msg.as_bytes())?;

    let mut reader = BufReader::new(&stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;

    let response = response.trim();
    if response.is_empty() {
        return Ok(None);
    }
    Ok(Some(response.to_string()))
}

/// Extrae el campo numérico `data` de una respuesta JSON de mpv.
fn mpv_response_data(response: &str) -> Option<f64> {
    let v: Value = serde_json::from_str(response).ok()?;
    v.get("data").and_then(|d| d.as_f64())
}

/// Construye el comando JSON para alternar pausa/reproducción.
fn mpv_cmd_toggle_pause() -> String {
    r#"{"command":["cycle","pause"]}"#.to_string()
}

/// Construye el comando JSON para establecer pausa (true) o reproducción (false).
fn mpv_cmd_set_pause(pause: bool) -> String {
    format!(r#"{{"command":["set_property","pause",{}]}}"#, pause)
}

/// Construye el comando JSON para establecer el volumen (0-100).
fn mpv_cmd_set_volume(volume: f64) -> String {
    format!(r#"{{"command":["set_property","volume",{}]}}"#, volume)
}

/// Construye el comando JSON para establecer la posición de tiempo (segundos).
fn mpv_cmd_set_time_pos(seconds: f64) -> String {
    format!(r#"{{"command":["set_property","time-pos",{}]}}"#, seconds)
}

/// Construye el comando JSON para obtener la duración.
fn mpv_cmd_get_duration() -> String {
    r#"{"command":["get_property","duration"]}"#.to_string()
}

/// Reproductor basado en `mpv` lanzado como proceso externo.
pub struct MpvPlayer {
    child: Option<Child>,
    socket_path: Option<String>,
    current_path: Option<String>,
    is_playing: bool,
    mpv_command: String,
}

impl MpvPlayer {
    /// Crea un reproductor mpv con el comando por defecto (`mpv`).
    pub fn new() -> Self {
        Self::with_command(MPV_COMMAND.to_string())
    }

    /// Crea un reproductor usando un comando personalizado (útil en tests).
    pub fn with_command(mpv_command: String) -> Self {
        Self {
            child: None,
            socket_path: None,
            current_path: None,
            is_playing: false,
            mpv_command,
        }
    }

    /// Carga y comienza a reproducir un archivo de vídeo con mpv.
    pub fn load_file(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.load_file_with_tracks(file_path, TrackSelection::default())
    }

    /// Carga un archivo aplicando una selección de pistas de audio/subtítulos.
    pub fn load_file_with_tracks(
        &mut self,
        file_path: &str,
        selection: TrackSelection,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.stop()?;

        // Valida que el archivo exista (mpv solo fallaría al abrirlo tras lanzar el proceso).
        let es_url = file_path.starts_with("http://") || file_path.starts_with("https://");
        if !es_url && file_path != "-" && !std::path::Path::new(file_path).exists() {
            return Err(format!("archivo no encontrado: {}", file_path).into());
        }

        let socket_path = format!(
            "{}/acryl-mpv-{}-{}.sock",
            MPV_SOCKET_DIR,
            std::process::id(),
            NEXT_SOCKET_ID.fetch_add(1, Ordering::Relaxed)
        );
        // Elimina socket viejo si quedó de una ejecución anterior.
        let _ = std::fs::remove_file(&socket_path);

        let mut child = Command::new(&self.mpv_command)
            .args(build_mpv_args_with_tracks(
                file_path,
                &socket_path,
                selection,
            ))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
            .map_err(|e| format!("no se pudo lanzar '{}': {}", self.mpv_command, e))?;

        // Espera a que mpv cree el socket IPC (máx. ~5 s).
        let mut waited = 0;
        while !std::path::Path::new(&socket_path).exists() && waited < MPV_SOCKET_TIMEOUT_MS / 100 {
            std::thread::sleep(Duration::from_millis(100));
            waited += 1;
        }
        if !std::path::Path::new(&socket_path).exists() {
            let _ = child.kill();
            return Err(format!("mpv no creó el socket IPC en {}", socket_path).into());
        }

        self.child = Some(child);
        self.socket_path = Some(socket_path.clone());
        self.current_path = Some(file_path.to_string());
        self.is_playing = true;
        Ok(())
    }

    /// Envía un comando JSON al socket IPC de mpv.
    fn send(&self, command_json: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let socket_path = self.socket_path.as_ref().ok_or("reproductor no iniciado")?;
        mpv_request(socket_path, command_json)
    }

    /// Alterna pausa/reproducción.
    pub fn toggle_pause(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.send(&mpv_cmd_toggle_pause())?;
        Ok(())
    }

    /// Establece pausa (`true`) o reproducción (`false`).
    pub fn set_pause(&mut self, pause: bool) -> Result<(), Box<dyn std::error::Error>> {
        self.send(&mpv_cmd_set_pause(pause))?;
        Ok(())
    }

    /// Reanuda la reproducción.
    pub fn play(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.set_pause(false)
    }

    /// Pausa la reproducción.
    pub fn pause(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.set_pause(true)
    }

    /// Detiene la reproducción y cierra el proceso mpv.
    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(path) = self.socket_path.take() {
            let _ = std::fs::remove_file(&path);
        }
        self.current_path = None;
        self.is_playing = false;
        Ok(())
    }

    /// Busca a una fracción de la duración (0.0 = inicio, 1.0 = final).
    pub fn seek_fraction(&mut self, fraction: f64) -> Result<(), Box<dyn std::error::Error>> {
        let duration = self.duration_seconds()?;
        let seconds = fraction.clamp(0.0, 1.0) * duration;
        self.send(&mpv_cmd_set_time_pos(seconds))?;
        Ok(())
    }

    /// Devuelve la duración del vídeo en segundos (0,0 si no se puede obtener).
    pub fn duration_seconds(&self) -> Result<f64, Box<dyn std::error::Error>> {
        let response = self.send(&mpv_cmd_get_duration())?;
        Ok(response
            .as_deref()
            .and_then(mpv_response_data)
            .unwrap_or(0.0))
    }

    /// Lee una propiedad de mpv a través del socket IPC.
    fn property(&self, name: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let command = serde_json::json!({ "command": ["get_property", name] }).to_string();
        let response = self.send(&command)?.ok_or("mpv no devolvió respuesta")?;
        let value: Value = serde_json::from_str(&response)?;
        Ok(value.get("data").cloned().unwrap_or(Value::Null))
    }

    /// Devuelve los identificadores de pista de audio y subtítulos que mpv
    /// tiene seleccionados. `None` significa que la pista está desactivada
    /// (o que no hay ninguna). Útil para verificar `--aid` / `--sid`.
    pub fn selected_track_ids(
        &self,
    ) -> Result<(Option<i64>, Option<i64>), Box<dyn std::error::Error>> {
        let audio = self.property("aid")?;
        let subtitle = self.property("sid")?;
        Ok((audio.as_i64(), subtitle.as_i64()))
    }

    /// Ajusta el volumen (0.0 = silencio, 1.0 = máximo).
    pub fn set_volume(&mut self, volume: f64) -> Result<(), Box<dyn std::error::Error>> {
        let volume = (volume.clamp(0.0, 1.0) * 100.0).round();
        self.send(&mpv_cmd_set_volume(volume))?;
        Ok(())
    }

    /// Ruta del archivo actualmente cargado.
    pub fn current_path(&self) -> Option<&str> {
        self.current_path.as_deref()
    }

    /// Indica si hay un fichero cargado (y por tanto un proceso mpv vivo).
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }
}

impl Drop for MpvPlayer {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

impl Default for MpvPlayer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Servicio de historial (capa de dominio)
// ---------------------------------------------------------------------------

/// Servicio de reproductor: gestiona el historial de reproducción.
pub struct PlayerService;

impl PlayerService {
    /// Registra la intención de reproducir un archivo de vídeo en el historial.
    pub fn open(
        db_path: &str,
        media_id: i64,
        file_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut conn = open_db(db_path)?;
        let mut media_repo = MediaRepository::new(&mut conn);

        // Registrar reproducción
        media_repo.record_playback(media_id, file_path, 0.0, 0.0)?;

        close_db(conn);

        // La reproducción real se delega a mpv desde la UI (proceso externo).
        eprintln!("Reproduciendo (mpv): {}", file_path);

        Ok(())
    }

    /// Obtiene la lista de archivos de vídeo de un medio.
    pub fn get_playable_files(
        db_path: &str,
        media_id: i64,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut conn = open_db(db_path)?;
        let mut media_repo = MediaRepository::new(&mut conn);
        let files = media_repo.list_files_by_media(media_id)?;
        close_db(conn);

        Ok(files.into_iter().map(|f| f.file_path).collect())
    }
}

/// Lista los archivos de vídeo de un medio.
pub fn list_media_files(
    db_path: &str,
    media_id: i64,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    PlayerService::get_playable_files(db_path, media_id)
}

/// Reproduce un archivo de vídeo (registra en el historial).
pub fn play_file(
    db_path: &str,
    media_id: i64,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    PlayerService::open(db_path, media_id, file_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_mpv_args_contiene_argumentos_esperados() {
        let args = build_mpv_args("/ruta/video.mkv", "/tmp/acryl-mpv-123.sock");
        assert_eq!(
            args,
            vec![
                "--force-window=immediate",
                "--input-ipc-server=/tmp/acryl-mpv-123.sock",
                "--idle=yes",
                "--fullscreen",
                "/ruta/video.mkv",
            ]
        );
    }

    #[test]
    fn build_mpv_launch_args_separa_opciones_de_la_ruta() {
        let args = build_mpv_launch_args("/ruta/con espacios/video.mkv");
        assert_eq!(
            args,
            vec![
                "--force-window=immediate",
                "--fullscreen",
                "--",
                "/ruta/con espacios/video.mkv",
            ]
        );
    }

    #[test]
    fn build_mpv_launch_args_con_pistas_convierte_a_ids_de_mpv() {
        let selection = TrackSelection {
            audio_stream_index: Some(1),
            subtitle_stream_index: Some(0),
            subtitles_disabled: false,
        };
        let args = build_mpv_launch_args_with_tracks("/ruta/video.mkv", selection);
        assert_eq!(
            args,
            vec![
                "--force-window=immediate",
                "--fullscreen",
                "--aid=2",
                "--sid=1",
                "--",
                "/ruta/video.mkv",
            ]
        );
    }

    #[test]
    fn build_mpv_launch_args_desactiva_subtitulos_con_sid_no() {
        let selection = TrackSelection {
            audio_stream_index: None,
            subtitle_stream_index: Some(1),
            subtitles_disabled: true,
        };
        let args = build_mpv_launch_args_with_tracks("/ruta/video.mkv", selection);
        assert_eq!(
            args,
            vec![
                "--force-window=immediate",
                "--fullscreen",
                "--sid=no",
                "--",
                "/ruta/video.mkv",
            ]
        );
    }

    #[test]
    fn build_mpv_args_with_tracks_incluye_pistas_antes_del_archivo() {
        let selection = TrackSelection {
            audio_stream_index: Some(0),
            subtitle_stream_index: Some(1),
            subtitles_disabled: false,
        };
        let args = build_mpv_args_with_tracks("/ruta/video.mkv", "/tmp/acryl.sock", selection);
        assert_eq!(
            args,
            vec![
                "--force-window=immediate",
                "--input-ipc-server=/tmp/acryl.sock",
                "--idle=yes",
                "--fullscreen",
                "--aid=1",
                "--sid=2",
                "/ruta/video.mkv",
            ]
        );
    }

    #[test]
    fn track_selection_automatica_no_anade_argumentos_de_pista() {
        let selection = TrackSelection::default();
        assert!(selection.is_automatic());
        let args = build_mpv_launch_args_with_tracks("/ruta/video.mkv", selection);
        assert_eq!(
            args,
            vec![
                "--force-window=immediate",
                "--fullscreen",
                "--",
                "/ruta/video.mkv",
            ]
        );
    }

    #[test]
    fn launch_mpv_falla_limpiamente_si_el_archivo_no_existe() {
        let resultado = launch_mpv("/ruta/que/no/existe.mp4");
        assert!(resultado.is_err());
    }

    #[test]
    fn mpv_cmd_toggle_pause_es_json_valido() {
        let cmd = mpv_cmd_toggle_pause();
        let v: Value = serde_json::from_str(&cmd).unwrap();
        // ciclo + pause
        let arr = v.get("command").and_then(|c| c.as_array()).unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0], "cycle");
        assert_eq!(arr[1], "pause");
    }

    #[test]
    fn mpv_cmd_set_pause_genera_json_correcto() {
        let cmd = mpv_cmd_set_pause(true);
        let v: Value = serde_json::from_str(&cmd).unwrap();
        let arr = v.get("command").and_then(|c| c.as_array()).unwrap();
        assert_eq!(arr[0], "set_property");
        assert_eq!(arr[1], "pause");
        assert_eq!(arr[2], true);

        let cmd = mpv_cmd_set_pause(false);
        let v: Value = serde_json::from_str(&cmd).unwrap();
        let arr = v.get("command").and_then(|c| c.as_array()).unwrap();
        assert_eq!(arr[2], false);
    }

    #[test]
    fn mpv_cmd_set_volume_genera_json_correcto() {
        let cmd = mpv_cmd_set_volume(75.0);
        let v: Value = serde_json::from_str(&cmd).unwrap();
        let arr = v.get("command").and_then(|c| c.as_array()).unwrap();
        assert_eq!(arr[0], "set_property");
        assert_eq!(arr[1], "volume");
        assert_eq!(arr[2], 75.0);
    }

    #[test]
    fn mpv_cmd_set_time_pos_genera_json_correcto() {
        let cmd = mpv_cmd_set_time_pos(10.5);
        let v: Value = serde_json::from_str(&cmd).unwrap();
        let arr = v.get("command").and_then(|c| c.as_array()).unwrap();
        assert_eq!(arr[0], "set_property");
        assert_eq!(arr[1], "time-pos");
        assert_eq!(arr[2], 10.5);
    }

    #[test]
    fn mpv_response_data_extrae_duracion() {
        let respuesta = r#"{"data":180.0,"error":"success"}"#;
        assert_eq!(mpv_response_data(respuesta), Some(180.0));

        let sin_data = r#"{"error":"success","data":null}"#;
        assert_eq!(mpv_response_data(sin_data), None);

        let invalido = "no es json";
        assert_eq!(mpv_response_data(invalido), None);
    }

    #[test]
    fn mpv_player_con_comando_personalizado_no_lanza_proceso() {
        // Con un comando que no existe, load_file debe fallar limpiamente sin pánico.
        let mut player = MpvPlayer::with_command("comando-inexistente-acryl".to_string());
        let resultado = player.load_file("/ruta/inexistente.mkv");
        assert!(resultado.is_err());
        assert!(!player.is_playing());
        assert!(player.current_path().is_none());
    }

    fn stream(
        stream_type: &str,
        index: i32,
        language: &str,
        title: Option<&str>,
        external_path: Option<&str>,
    ) -> crate::db::media_repo::MediaStream {
        crate::db::media_repo::MediaStream {
            id: 0,
            media_id: 1,
            file_path: "/tmp/video.mkv".to_string(),
            stream_type: stream_type.to_string(),
            stream_index: index,
            language: Some(language.to_string()),
            codec: Some("aac".to_string()),
            title: title.map(str::to_string),
            external_path: external_path.map(str::to_string),
        }
    }

    #[test]
    fn completa_la_reproduccion_al_llegar_al_final() {
        assert!(should_complete_playback("series", 299.0, 300.0));
        assert!(should_complete_playback("series", 300.0, 300.0));
        assert!(should_complete_playback("series", 301.0, 300.0));
        assert!(!should_complete_playback("series", 0.0, 300.0));
        assert!(!should_complete_playback("series", 100.0, 1000.0));
        assert!(!should_complete_playback("series", 0.0, 0.0));
    }

    #[test]
    fn las_peliculas_usan_un_umbral_mayor() {
        assert!(!should_complete_playback("movie", 0.0, 600.0));
        assert!(should_complete_playback("movie", 0.0, 599.0));
    }

    #[test]
    fn modo_de_ventana_no_fuerza_pantalla_completa_en_ventana_normal() {
        let args = build_mpv_args_for_playback(
            "/ruta/video.mkv",
            "/tmp/acryl.sock",
            &ResolvedTracks::default(),
            PlayerWindowMode::Normal,
        );
        assert!(!args.iter().any(|arg| arg == "--fullscreen"));
        assert!(!args.iter().any(|arg| arg.starts_with("--window-maximized")));
    }

    #[test]
    fn modo_de_ventana_distingue_maximizado_de_pantalla_completa() {
        let maximized = build_mpv_launch_args_for_playback(
            "/ruta/video.mkv",
            &ResolvedTracks::default(),
            PlayerWindowMode::Maximized,
        );
        assert!(maximized.contains(&"--window-maximized=yes".to_string()));
        assert!(!maximized.contains(&"--fullscreen".to_string()));

        let fullscreen = build_mpv_launch_args_for_playback(
            "/ruta/video.mkv",
            &ResolvedTracks::default(),
            PlayerWindowMode::Fullscreen,
        );
        assert!(fullscreen.contains(&"--fullscreen".to_string()));
        assert!(
            !fullscreen
                .iter()
                .any(|arg| arg.starts_with("--window-maximized"))
        );
    }

    #[test]
    fn subtitulos_externos_se_pasan_con_sub_file_y_sid_determinista() {
        let tracks = ResolvedTracks {
            selection: TrackSelection {
                audio_stream_index: None,
                subtitle_stream_index: Some(2),
                subtitles_disabled: false,
            },
            external_subtitles: vec![
                "/tmp/video.es.srt".to_string(),
                "/tmp/video.en.srt".to_string(),
            ],
        };
        let args =
            build_mpv_launch_args_for_playback("/tmp/video.mkv", &tracks, PlayerWindowMode::Normal);
        assert!(args.contains(&"--sub-auto=no".to_string()));
        let sub_files: Vec<String> = args
            .iter()
            .filter(|arg| arg.starts_with("--sub-file="))
            .cloned()
            .collect();
        assert_eq!(
            sub_files,
            vec![
                "--sub-file=/tmp/video.es.srt".to_string(),
                "--sub-file=/tmp/video.en.srt".to_string(),
            ]
        );
        assert!(args.contains(&"--sid=3".to_string()));
    }

    #[test]
    fn preferencia_por_clave_ignora_un_indice_que_apunta_a_otra_pista() {
        let streams = vec![
            stream("audio", 0, "spa", None, None),
            stream("audio", 1, "eng", None, None),
            stream("subtitle", 0, "spa", None, None),
            stream("subtitle", 1, "eng", None, None),
        ];
        let mut preference = TrackPreference {
            selection: TrackSelection {
                audio_stream_index: Some(0),
                subtitle_stream_index: Some(0),
                subtitles_disabled: false,
            },
            ..Default::default()
        };
        preference.audio_track_key = Some(streams[1].track_key());
        preference.subtitle_track_key = Some(streams[3].track_key());

        let resolved = resolve_track_preference(&preference, &streams);
        assert_eq!(resolved.selection.audio_stream_index, Some(1));
        assert_eq!(resolved.selection.subtitle_stream_index, Some(1));
        assert!(!resolved.selection.subtitles_disabled);
    }

    #[test]
    fn preferencia_de_pista_inexistente_no_reutiliza_otra_pista() {
        let streams = vec![stream("subtitle", 0, "spa", None, None)];
        let preference = TrackPreference {
            selection: TrackSelection {
                audio_stream_index: None,
                subtitle_stream_index: Some(0),
                subtitles_disabled: false,
            },
            subtitle_track_key: Some("embedded:subtitle:eng:aac:".to_string()),
            ..Default::default()
        };

        let resolved = resolve_track_preference(&preference, &streams);
        assert_eq!(resolved.selection.subtitle_stream_index, None);
        assert!(resolved.selection.subtitles_disabled);
    }

    #[test]
    fn preferencia_de_sidecar_conserva_su_ruta_como_identidad() {
        let external = stream("subtitle", 1, "spa", Some("es"), Some("/tmp/video.es.srt"));
        let streams = vec![stream("subtitle", 0, "eng", None, None), external.clone()];
        let preference = TrackPreference {
            selection: TrackSelection {
                audio_stream_index: None,
                subtitle_stream_index: None,
                subtitles_disabled: false,
            },
            subtitle_track_key: Some(external.track_key()),
            subtitle_external_path: external.external_path.clone(),
            ..Default::default()
        };

        let resolved = resolve_track_preference(&preference, &streams);
        assert_eq!(resolved.selection.subtitle_stream_index, Some(1));
        assert_eq!(
            resolved.external_subtitles,
            vec!["/tmp/video.es.srt".to_string()]
        );
    }
}
