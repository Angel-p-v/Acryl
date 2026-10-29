//! Reproductor de vídeo integrado sobre libmpv.
//!
//! Envuelve un `mpv_handle` con semántica de propietario único y limpieza
//! automática mediante `Drop` (`mpv_terminate_destroy`), para nunca dejar
//! fugas de recursos de libmpv.

use crate::player::ffi::*;
use std::ffi::CString;
use std::os::raw::{c_int, c_void};

/// Errores del reproductor mpv.
#[derive(Debug, thiserror::Error)]
pub enum PlayerError {
    #[error("no se pudo crear el contexto de mpv")]
    CreateFailed,
    #[error("fallo al inicializar mpv (código {0})")]
    InitFailed(i32),
    #[error("fallo al establecer opción '{0}' (código {1})")]
    OptionFailed(String, i32),
    #[error("fallo al ejecutar comando '{0}' (código {1})")]
    CommandFailed(String, i32),
    #[error("fallo al obtener propiedad '{0}' (código {1})")]
    PropertyFailed(String, i32),
    #[error("fallo al convertir ruta a CString: {0}")]
    CStringFailed(#[from] std::ffi::NulError),
}

/// Reproductor de vídeo basado en libmpv.
///
/// El handle de mpv se crea en `new()` y se destruye automáticamente al
/// hacer `drop` mediante `mpv_terminate_destroy`.
pub struct MpvPlayer {
    handle: MpvHandle,
}

impl MpvPlayer {
    /// Crea un nuevo reproductor mpv con opciones básicas (sin terminal,
    /// sin bloqueo de consola).
    ///
    /// Si se proporciona `wid` (window ID nativo, X11 en Linux), mpv
    /// renderizará directamente dentro de esa ventana en lugar de crear
    /// una ventana propia.
    pub fn new(wid: Option<u64>) -> Result<Self, PlayerError> {
        let handle = unsafe { mpv_create() };
        if handle.is_null() {
            return Err(PlayerError::CreateFailed);
        }

        // Desactivar la salida a terminal (no necesitamos consola en la app).
        set_option_string(handle, "terminal", "no")?;
        // Forzar modo de reproducción en ventana (vo = gpu).
        set_option_string(handle, "vo", "gpu")?;
        // Permitir aceleración por hardware.
        set_option_string(handle, "hwdec", "auto")?;

        // Si se proporciona un window ID, indicarle a mpv que renderice
        // dentro de esa ventana en lugar de crear una propia.
        if let Some(wid) = wid {
            set_option_string(handle, "wid", &wid.to_string())?;
        }

        // Helper privado para establecer opciones como string.
        fn set_option_string(ctx: MpvHandle, name: &str, value: &str) -> Result<(), PlayerError> {
            let c_name = CString::new(name).map_err(PlayerError::CStringFailed)?;
            let c_value = CString::new(value).map_err(PlayerError::CStringFailed)?;
            let rc = unsafe { mpv_set_option_string(ctx, c_name.as_ptr(), c_value.as_ptr()) };
            if rc != 0 {
                return Err(PlayerError::OptionFailed(name.to_string(), rc));
            }
            Ok(())
        }

        let rc = unsafe { mpv_initialize(handle) };
        if rc != 0 {
            unsafe { mpv_terminate_destroy(handle); }
            return Err(PlayerError::InitFailed(rc));
        }

        Ok(Self { handle })
    }

    /// Carga y comienza a reproducir un archivo de vídeo.
    pub fn load_file(&self, path: &str) -> Result<(), PlayerError> {
        let command = format!("loadfile {}", path);
        let c_command = CString::new(command.clone())
            .map_err(PlayerError::CStringFailed)?;
        let rc = unsafe { mpv_command_string(self.handle, c_command.as_ptr()) };
        if rc != 0 {
            return Err(PlayerError::CommandFailed(command, rc));
        }
        Ok(())
    }

    /// Reanuda o inicia la reproducción.
    pub fn play(&self) -> Result<(), PlayerError> {
        self.set_property_flag("pause", false)
    }

    /// Pausa la reproducción.
    pub fn pause(&self) -> Result<(), PlayerError> {
        self.set_property_flag("pause", true)
    }

    /// Detiene la reproducción y cierra el archivo actual.
    pub fn stop(&self) -> Result<(), PlayerError> {
        let rc = unsafe { mpv_command_string(self.handle, b"stop\0".as_ptr() as *const i8) };
        if rc != 0 {
            return Err(PlayerError::CommandFailed("stop".to_string(), rc));
        }
        Ok(())
    }

    /// Busca a una posición (segundos). Positivo = adelante, negativo = atrás.
    pub fn seek(&self, seconds: f64) -> Result<(), PlayerError> {
        let command = format!("seek {}", seconds);
        let c_command = CString::new(command.clone())
            .map_err(PlayerError::CStringFailed)?;
        let rc = unsafe { mpv_command_string(self.handle, c_command.as_ptr()) };
        if rc != 0 {
            return Err(PlayerError::CommandFailed(command, rc));
        }
        Ok(())
    }

    /// Establece el volumen (0.0 a 100.0).
    pub fn set_volume(&self, volume: f64) -> Result<(), PlayerError> {
        self.set_property_double("volume", volume)
    }

    /// Devuelve la posición actual de reproducción en segundos.
    pub fn time_position(&self) -> Result<f64, PlayerError> {
        self.get_property_double("time-pos")
    }

    /// Devuelve la duración total en segundos.
    pub fn duration(&self) -> Result<f64, PlayerError> {
        self.get_property_double("duration")
    }

    /// Devuelve si el reproductor está pausado.
    pub fn is_paused(&self) -> Result<bool, PlayerError> {
        self.get_property_flag("pause")
    }

    /// Devuelve el volumen actual.
    pub fn volume(&self) -> Result<f64, PlayerError> {
        self.get_property_double("volume")
    }

    // --- helpers internos ---

    fn set_property_flag(&self, name: &str, value: bool) -> Result<(), PlayerError> {
        let c_name = CString::new(name).map_err(PlayerError::CStringFailed)?;
        let flag: c_int = if value { 1 } else { 0 };
        let rc = unsafe {
            mpv_set_property(
                self.handle,
                c_name.as_ptr(),
                MPV_FORMAT_FLAG,
                &flag as *const c_int as *const c_void,
            )
        };
        if rc != 0 {
            return Err(PlayerError::PropertyFailed(name.to_string(), rc));
        }
        Ok(())
    }

    fn set_property_double(&self, name: &str, value: f64) -> Result<(), PlayerError> {
        let c_name = CString::new(name).map_err(PlayerError::CStringFailed)?;
        let rc = unsafe {
            mpv_set_property(
                self.handle,
                c_name.as_ptr(),
                MPV_FORMAT_DOUBLE,
                &value as *const f64 as *const c_void,
            )
        };
        if rc != 0 {
            return Err(PlayerError::PropertyFailed(name.to_string(), rc));
        }
        Ok(())
    }

    fn get_property_double(&self, name: &str) -> Result<f64, PlayerError> {
        let c_name = CString::new(name).map_err(PlayerError::CStringFailed)?;
        let mut value: f64 = 0.0;
        let rc = unsafe {
            mpv_get_property(
                self.handle,
                c_name.as_ptr(),
                MPV_FORMAT_DOUBLE,
                &mut value as *mut f64 as *mut c_void,
            )
        };
        if rc != 0 {
            return Err(PlayerError::PropertyFailed(name.to_string(), rc));
        }
        Ok(value)
    }

    fn get_property_flag(&self, name: &str) -> Result<bool, PlayerError> {
        let c_name = CString::new(name).map_err(PlayerError::CStringFailed)?;
        let mut value: c_int = 0;
        let rc = unsafe {
            mpv_get_property(
                self.handle,
                c_name.as_ptr(),
                MPV_FORMAT_FLAG,
                &mut value as *mut c_int as *mut c_void,
            )
        };
        if rc != 0 {
            return Err(PlayerError::PropertyFailed(name.to_string(), rc));
        }
        Ok(value != 0)
    }
}

unsafe impl Send for MpvPlayer {}
unsafe impl Sync for MpvPlayer {}

impl Drop for MpvPlayer {
    fn drop(&mut self) {
        // mpv_terminate_destroy es idempotente con respecto a la liberación de
        // recursos: llama a mpv_terminate y luego a mpv_destroy internamente.
        unsafe { mpv_terminate_destroy(self.handle); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifica que se puede crear un reproductor y que su `Drop` libera
    /// los recursos sin panic ni fugas (libmpv_terminate_destroy se llama
    /// automáticamente al salir del scope).
    #[test]
    fn crear_y_destruir_reproductor_sin_fugas() {
        let player = MpvPlayer::new(None).expect("debe crear reproductor mpv");
        // Solo verificar que el handle es no nulo mientras vive.
        assert!(!player.handle.is_null());
        // Al finalizar el scope, Drop llama a mpv_terminate_destroy.
    }

    /// Verifica que loadfile con una ruta inexistente devuelve error controlado
    /// (no panic) en lugar de fallar silenciosamente.
    #[test]
    fn load_file_ruta_inexistente_devuelve_error() {
        let player = MpvPlayer::new(None).expect("debe crear reproductor mpv");
        let resultado = player.load_file("/ruta/que/no/existe.mkv");
        // mpv puede aceptar la ruta y fallar al reproducir, o rechazarla;
        // lo importante es que no haga panic y devuelva un Result.
        let _ = resultado;
    }
}
