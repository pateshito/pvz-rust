//! Modo de partida: tablero, soles, semillas, zombies y guisantes.
//!
//! - `board`, `defs`, `level`, `sim`: lógica pura (sin Bevy), con pruebas.
//! - `view`: lo que se ve y se toca en el estado `Jugar`.

// Campos de la simulación que la vista todavía no consulta (vida, fase de disparo…).
#![allow(dead_code)]

pub mod board;
pub mod defs;
pub mod level;
pub mod sim;
pub mod view;
