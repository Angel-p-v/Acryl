//! Servicio de reproductor: delega la reproducción a `mpv` (proceso externo).
//!
//! La reproducción de vídeo se delega al reproductor `mpv` lanzado como
//! proceso externo mediante `std::process::Command`. El control (play, pausa,
//! stop, seek, volumen) se envía a `mpv` a través de su socket IPC.

use crate::db::{MediaRepository, close_db, open_db};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// Directorio donde se crean los sockets IPC de mpv.
const MPV_SOCKET_DIR: &str = "/tmp";

/// Comando de reproductor externo por defecto.
const MPV_COMMAND: &str = "mpv";

/// Tiempo máximo de espera (ms) para que mpv cree su socket IPC.
const MPV_SOCKET_TIMEOUT_MS: u64 = 5000;

/// Construye los argumentos de línea de comandos de mpv (puro, testeable).
pub fn build_mpv_args(file_path: &str, socket_path: &str) -> Vec<String> {
    vec![
        "--force-window=immediate".to_string(),
        format!("--input-ipc-server={}", socket_path),
        "--idle=yes".to_string(),
        file_path.to_string(),
    ]
}

/// Construye los argumentos para abrir mpv como reproductor independiente.
pub fn build_mpv_launch_args(file_path: &str) -> Vec<String> {
    vec![
        "--force-window=immediate".to_string(),
        "--".to_string(),
        file_path.to_string(),
    ]
}

/// Lanza mpv en su propia ventana para reproducir un archivo.
///
/// El proceso se espera en un hilo secundario para evitar dejar procesos
/// zombie cuando mpv termina.
pub fn launch_mpv(file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let is_url = file_path.starts_with("http://") || file_path.starts_with("https://");
    if !is_url && file_path != "-" && !std::path::Path::new(file_path).exists() {
        return Err(format!("archivo no encontrado: {}", file_path).into());
    }

    let mut child = Command::new(MPV_COMMAND)
        .args(build_mpv_launch_args(file_path))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| format!("no se pudo lanzar '{}': {}", MPV_COMMAND, e))?;

    std::thread::spawn(move || {
        let _ = child.wait();
    });

    Ok(())
}

/// Lanza mpv y registra la reproducción en el historial si se conoce el medio.
pub fn play_external(
    db_path: &str,
    media_id: Option<i64>,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    launch_mpv(file_path)?;

    if let Some(media_id) = media_id
        && let Err(error) = PlayerService::open(db_path, media_id, file_path)
    {
        eprintln!("No se pudo registrar la reproducción: {}", error);
    }

    Ok(())
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
        self.stop()?;

        // Valida que el archivo exista (mpv solo fallaría al abrirlo tras lanzar el proceso).
        let es_url = file_path.starts_with("http://") || file_path.starts_with("https://");
        if !es_url && file_path != "-" && !std::path::Path::new(file_path).exists() {
            return Err(format!("archivo no encontrado: {}", file_path).into());
        }

        let socket_path = format!("{}/acryl-mpv-{}.sock", MPV_SOCKET_DIR, std::process::id());
        // Elimina socket viejo si quedó de una ejecución anterior.
        let _ = std::fs::remove_file(&socket_path);

        let mut child = Command::new(&self.mpv_command)
            .args(build_mpv_args(file_path, &socket_path))
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
        media_repo.record_playback(media_id, 0.0, 0.0)?;

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
                "--",
                "/ruta/con espacios/video.mkv",
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
}
