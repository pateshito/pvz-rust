//! Modo de partida: tablero, soles, semillas, zombies y guisantes.
//!
//! - `board`, `defs`, `level`, `sim`: lógica pura (sin Bevy), con pruebas.
//! - el resto: lo que se ve y se toca (se añade en las siguientes etapas).

// Mientras la parte visual no use todo lo definido aquí.
#![allow(dead_code)]

pub mod board;
pub mod defs;
pub mod level;
pub mod sim;
