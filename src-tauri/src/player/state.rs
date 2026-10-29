//! Estado del reproductor gestionado por Tauri.
//!
//! Almacena el `MpvPlayer` activo dentro de un `Mutex` para que los
//! comandos Tauri puedan acceder a él de forma segura desde cualquier hilo.

use crate::player::mpv::MpvPlayer;
use std::sync::Mutex;

/// Estado del reproductor compartido por la aplicación Tauri.
pub struct PlayerState {
    pub player: Mutex<Option<MpvPlayer>>,
}

impl PlayerState {
    pub fn new() -> Self {
        Self {
            player: Mutex::new(None),
        }
    }
}
