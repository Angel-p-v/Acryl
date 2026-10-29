//! Módulo de reproductor integrado.
//!
//! Proporciona `MpvPlayer`, un envoltorio seguro sobre libmpv con limpieza
//! automática de recursos mediante `Drop`.

pub mod ffi;
pub mod mpv;

pub use mpv::{MpvPlayer, PlayerError};
