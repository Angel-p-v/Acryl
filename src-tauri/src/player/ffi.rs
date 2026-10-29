//! Declaraciones FFI a libmpv (API C).
//!
//! Solo se declaran las funciones que usamos. `mpv_handle` es opaco para
//! nosotros, por lo que lo modelamos como un puntero raw a `c_void`.

use std::os::raw::{c_char, c_double, c_int, c_void};

/// Handle opaco a una instancia de mpv.
pub type MpvHandle = *mut c_void;

/// Códigos de evento de mpv (subconjunto que nos interesa).
pub const MPV_EVENT_NONE: u32 = 0;
pub const MPV_EVENT_SHUTDOWN: u32 = 1;
pub const MPV_EVENT_LOG_MESSAGE: u32 = 2;
pub const MPV_EVENT_START_FILE: u32 = 9;
pub const MPV_EVENT_END_FILE: u32 = 10;
pub const MPV_EVENT_FILE_LOADED: u32 = 12;
pub const MPV_EVENT_PAUSE: u32 = 22;
pub const MPV_EVENT_UNPAUSE: u32 = 23;
pub const MPV_EVENT_SEEK: u32 = 26;
pub const MPV_EVENT_PROPERTY_CHANGE: u32 = 31;

/// Formatos de propiedad de mpv.
pub const MPV_FORMAT_NONE: c_int = 0;
pub const MPV_FORMAT_STRING: c_int = 1;
pub const MPV_FORMAT_INT64: c_int = 2;
pub const MPV_FORMAT_DOUBLE: c_int = 3;
pub const MPV_FORMAT_FLAG: c_int = 4;
pub const MPV_FORMAT_NODE: c_int = 11;

/// Evento de mpv (layout C, coincidente con la estructura C de libmpv).
#[repr(C)]
pub struct MpvEvent {
    pub event_id: u32,
    pub error: c_int,
    pub reply_userdata: c_int,
    pub data: *mut c_void,
}

unsafe extern "C" {
    /// Crea un nuevo contexto de mpv.
    pub fn mpv_create() -> MpvHandle;

    /// Destruye el contexto y termina cualquier reproducción en curso.
    pub fn mpv_terminate_destroy(ctx: MpvHandle);

    /// Establece una opción como string.
    pub fn mpv_set_option_string(
        ctx: MpvHandle,
        name: *const c_char,
        value: *const c_char,
    ) -> c_int;

    /// Inicializa el contexto tras establecer opciones.
    pub fn mpv_initialize(ctx: MpvHandle) -> c_int;

    /// Ejecuta un comando como string (ej. "loadfile /path/to/file.mkv").
    pub fn mpv_command_string(ctx: MpvHandle, command: *const c_char) -> c_int;

    /// Habilita/deshabilita la recepción de un evento.
    pub fn mpv_request_event(ctx: MpvHandle, event_id: u32, enable: c_int) -> c_int;

    /// Espera un evento (bloqueante con timeout en segundos).
    pub fn mpv_wait_event(ctx: MpvHandle, timeout: c_double) -> *const MpvEvent;

    /// Obtiene el valor de una propiedad.
    pub fn mpv_get_property(
        ctx: MpvHandle,
        name: *const c_char,
        format: c_int,
        result: *mut c_void,
    ) -> c_int;

    /// Establece el valor de una propiedad.
    pub fn mpv_set_property(
        ctx: MpvHandle,
        name: *const c_char,
        format: c_int,
        data: *const c_void,
    ) -> c_int;

    /// Activa el sistema de logs de mpv.
    pub fn mpv_request_log(ctx: MpvHandle, min_level: c_int);
}
