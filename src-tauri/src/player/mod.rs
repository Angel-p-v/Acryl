//! Módulo de reproductor integrado.
//!
//! Proporciona `MpvPlayer`, un envoltorio seguro sobre libmpv con limpieza
//! automática de recursos mediante `Drop`.

pub mod ffi;
pub mod mpv;
pub mod state;

pub use mpv::MpvPlayer;
pub use state::PlayerState;
