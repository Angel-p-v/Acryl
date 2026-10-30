//! Test de integración del reproductor `mpv` (proceso externo).
//!
//! Requiere que `mpv` y `ffmpeg` estén instalados y que exista una sesión
//! gráfica (Wayland o X11). Se saltea automáticamente si faltan.

use acryl::core::player::MpvPlayer;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Ruta del ejecutor mpv.
const MPV: &str = "mpv";

/// Comprueba si un ejecutable está disponible en el PATH.
fn disponible(bin: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {}", bin))
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Genera un vídeo de prueba corto (3 s) con ffmpeg y devuelve su ruta.
fn generar_video_prueba(dir: &std::path::Path) -> std::path::PathBuf {
    let salida = dir.join("prueba.mp4");
    let estado = Command::new("ffmpeg")
        .arg("-y")
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg("testsrc=duration=3:size=320x240:rate=30")
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg("sine=frequency=440:duration=3")
        .arg("-c:v")
        .arg("libx264")
        .arg("-c:a")
        .arg("aac")
        .arg("-shortest")
        .arg("-loglevel")
        .arg("error")
        .arg(&salida)
        .status()
        .expect("ejecutar ffmpeg para generar video de prueba");
    assert!(
        estado.success(),
        "ffmpeg no pudo generar el video de prueba"
    );
    salida
}

fn sesion_grafica_disponible() -> bool {
    std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok()
}

#[test]
fn reproductor_mpv_carga_y_controla_video_real() {
    if !disponible(MPV) || !disponible("ffmpeg") || !sesion_grafica_disponible() {
        eprintln!("test salteado: se requiere mpv, ffmpeg y sesión gráfica");
        return;
    }

    let dir = tempfile::tempdir().expect("crear directorio temporal");
    let video = generar_video_prueba(dir.path());

    let mut reproductor = MpvPlayer::new();

    // Carga el vídeo.
    reproductor
        .load_file(video.to_str().unwrap())
        .expect("mpv debería cargar el vídeo");
    assert!(reproductor.is_playing(), "debería estar reproduciendo");
    assert_eq!(
        reproductor.current_path(),
        Some(video.to_str().unwrap()),
        "ruta cargada debe coincidir"
    );

    // Espera un poco a que mpv inicie y reporte duración.
    thread::sleep(Duration::from_millis(800));

    let duracion = reproductor.duration_seconds().expect("obtener duración");
    assert!(
        duracion > 0.0,
        "la duración debería ser mayor a cero, fue {}",
        duracion
    );

    // Pausa y reanuda.
    reproductor.pause().expect("pausar");
    reproductor.play().expect("reanudar");

    // Ajusta volumen.
    reproductor.set_volume(0.5).expect("ajustar volumen");

    // Seek a la mitad.
    reproductor.seek_fraction(0.5).expect("hacer seek");

    // Detén.
    reproductor.stop().expect("detener");
    assert!(!reproductor.is_playing(), "no debería estar reproduciendo");
    assert!(reproductor.current_path().is_none(), "ruta debe limpiarse");

    // `Drop` limpia el proceso y el socket automáticamente.
    drop(reproductor);
}

#[test]
fn reproductor_mpv_sin_archivo_falla_limpio() {
    if !disponible(MPV) {
        eprintln!("test salteado: se requiere mpv");
        return;
    }

    let mut reproductor = MpvPlayer::new();
    let resultado = reproductor.load_file("/ruta/que/no/existe.mp4");
    assert!(
        resultado.is_err(),
        "cargar archivo inexistente debería fallar"
    );
    assert!(!reproductor.is_playing());
    assert!(reproductor.current_path().is_none());
}
