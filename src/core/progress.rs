//! Cálculo y formateo del progreso de reproducción.
//!
//! Convierte la posición y duración guardadas en `playback_history` en una
//! fracción usable por `GtkProgressBar` y en textos legibles para la UI del
//! historial.

/// Resultado del cálculo de progreso de una reproducción.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaybackProgress {
    /// Fracción entre 0.0 y 1.0 lista para `ProgressBar::set_fraction`.
    pub fraction: f64,
    /// Porcentaje entero entre 0 y 100.
    pub percent: u32,
    /// Segundos restantes, si se conoce la duración.
    pub remaining_seconds: Option<f64>,
    /// Indica si la reproducción se marcó como completada.
    pub completed: bool,
}

/// Umbral a partir del cual se considera que un vídeo está visto.
const COMPLETED_THRESHOLD: f64 = 0.95;

/// Calcula el progreso de una reproducción a partir de su posición y duración.
///
/// Reglas:
/// - Si `completed` es true, el progreso es del 100 % sin importar los demás
///   valores.
/// - Si la duración es válida (> 0), la fracción es
///   `(position / duration).clamp(0.0, 1.0)`.
/// - Si la duración no es válida y la posición es > 0, hay progreso pero sin
///   referencia: fracción 0, porcentaje 0, restante `None`.
/// - Valores negativos, NaN o infinitos se tratan como 0.
pub fn compute_playback_progress(
    position: f64,
    duration: f64,
    completed: bool,
) -> PlaybackProgress {
    let finite_position = if position.is_finite() && position > 0.0 {
        position
    } else {
        0.0
    };
    let finite_duration = if duration.is_finite() && duration > 0.0 {
        duration
    } else {
        0.0
    };

    if completed {
        return PlaybackProgress {
            fraction: 1.0,
            percent: 100,
            remaining_seconds: Some(0.0),
            completed: true,
        };
    }

    if finite_duration > 0.0 {
        let fraction = (finite_position / finite_duration).clamp(0.0, 1.0);
        let percent = (fraction * 100.0).round() as u32;
        let remaining = Some((finite_duration - finite_position).max(0.0));
        PlaybackProgress {
            fraction,
            percent,
            remaining_seconds: remaining,
            completed: fraction >= COMPLETED_THRESHOLD,
        }
    } else {
        // No hay duración de referencia: progreso desconocido.
        PlaybackProgress {
            fraction: 0.0,
            percent: 0,
            remaining_seconds: None,
            completed: false,
        }
    }
}

/// Formatea el tiempo restante en segundos en un texto legible.
///
/// Ejemplos: "8 min", "1 h 12 min", "45 s".
pub fn format_remaining(seconds: f64) -> String {
    let secs = seconds.round() as i64;
    if secs <= 0 {
        return "0 s".to_string();
    }
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let remaining_seconds = secs % 60;

    if hours > 0 {
        if minutes > 0 {
            if remaining_seconds > 0 {
                format!("{} h {} min {} s", hours, minutes, remaining_seconds)
            } else {
                format!("{} h {} min", hours, minutes)
            }
        } else if remaining_seconds > 0 {
            format!("{} h {} s", hours, remaining_seconds)
        } else {
            format!("{} h", hours)
        }
    } else if minutes > 0 {
        if remaining_seconds > 0 {
            format!("{} min {} s", minutes, remaining_seconds)
        } else {
            format!("{} min", minutes)
        }
    } else {
        format!("{} s", remaining_seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sin_progreso_es_cero_por_ciento() {
        let p = compute_playback_progress(0.0, 3600.0, false);
        assert_eq!(p.fraction, 0.0);
        assert_eq!(p.percent, 0);
        assert_eq!(p.remaining_seconds, Some(3600.0));
        assert!(!p.completed);
    }

    #[test]
    fn mitad_de_reproduccion() {
        let p = compute_playback_progress(1800.0, 3600.0, false);
        assert_eq!((p.fraction * 100.0).round(), 50.0);
        assert_eq!(p.percent, 50);
        assert_eq!(p.remaining_seconds, Some(1800.0));
        assert!(!p.completed);
    }

    #[test]
    fn completado_marca_cien_por_ciento() {
        let p = compute_playback_progress(1200.0, 3600.0, true);
        assert_eq!(p.fraction, 1.0);
        assert_eq!(p.percent, 100);
        assert_eq!(p.remaining_seconds, Some(0.0));
        assert!(p.completed);
    }

    #[test]
    fn posicion_superior_a_duracion_se_acota_al_cien_por_ciento() {
        let p = compute_playback_progress(4000.0, 3600.0, false);
        assert_eq!(p.fraction, 1.0);
        assert_eq!(p.percent, 100);
        assert_eq!(p.remaining_seconds, Some(0.0));
    }

    #[test]
    fn duracion_desconocida_progreso_desconocido() {
        let p = compute_playback_progress(1200.0, 0.0, false);
        assert_eq!(p.fraction, 0.0);
        assert_eq!(p.percent, 0);
        assert_eq!(p.remaining_seconds, None);
        assert!(!p.completed);
    }

    #[test]
    fn valores_negativos_no_rompen_el_calculo() {
        let p = compute_playback_progress(-100.0, 3600.0, false);
        assert_eq!(p.fraction, 0.0);
        assert_eq!(p.percent, 0);
        assert!(!p.completed);
    }

    #[test]
    fn nan_es_tratado_como_sin_progreso() {
        let p = compute_playback_progress(f64::NAN, f64::NAN, false);
        assert_eq!(p.fraction, 0.0);
        assert_eq!(p.percent, 0);
        assert_eq!(p.remaining_seconds, None);
        assert!(!p.completed);
    }

    #[test]
    fn infinito_es_tratado_como_sin_progreso() {
        let p = compute_playback_progress(f64::INFINITY, 3600.0, false);
        assert_eq!(p.fraction, 0.0);
        assert_eq!(p.percent, 0);
        assert_eq!(p.remaining_seconds, Some(3600.0));
        assert!(!p.completed);
    }

    #[test]
    fn formato_restante_horas_y_minutos() {
        assert_eq!(format_remaining(3661.0), "1 h 1 min 1 s");
        assert_eq!(format_remaining(3600.0), "1 h");
        assert_eq!(format_remaining(60.0), "1 min");
        assert_eq!(format_remaining(95.0), "1 min 35 s");
        assert_eq!(format_remaining(30.0), "30 s");
        assert_eq!(format_remaining(0.0), "0 s");
    }
}
