//! Medidas del tablero. Todo en "coordenadas de pantalla" del juego original:
//! 800x600, origen arriba a la izquierda, y hacia abajo.

pub const ROWS: usize = 5;
pub const COLS: usize = 9;

pub const SCREEN_W: f32 = 800.0;
pub const SCREEN_H: f32 = 600.0;

pub const CELL_W: f32 = 80.0;
pub const CELL_H: f32 = 100.0;
/// Esquina superior izquierda del césped.
pub const LAWN_LEFT: f32 = 40.0;
pub const LAWN_TOP: f32 = 80.0;

/// `background1.jpg` mide 1400x600; la ventana de 800x600 se ve desde esta x.
pub const BACKGROUND_OFFSET_X: f32 = 220.0;

/// Si el frente de un zombie cruza esta x, los zombies entran a la casa.
pub const LOSE_X: f32 = 0.0;

pub fn cell_left(col: usize) -> f32 {
    LAWN_LEFT + CELL_W * col as f32
}

pub fn cell_top(row: usize) -> f32 {
    LAWN_TOP + CELL_H * row as f32
}

/// Coordenadas de pantalla (origen arriba a la izquierda, y hacia abajo) al mundo
/// de una cámara 2D de Bevy centrada en una ventana de 800×600 (y hacia arriba).
pub fn screen_to_world(x: f32, y: f32) -> (f32, f32) {
    (x - SCREEN_W * 0.5, SCREEN_H * 0.5 - y)
}

/// Celda `(fila, columna)` bajo un punto de pantalla, si está dentro del césped.
pub fn cell_at(x: f32, y: f32) -> Option<(usize, usize)> {
    let fx = (x - LAWN_LEFT) / CELL_W;
    let fy = (y - LAWN_TOP) / CELL_H;
    if fx < 0.0 || fy < 0.0 {
        return None;
    }
    let (col, row) = (fx as usize, fy as usize);
    (row < ROWS && col < COLS).then_some((row, col))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn celdas_y_bordes() {
        assert_eq!(cell_at(41.0, 81.0), Some((0, 0)));
        assert_eq!(cell_at(759.0, 579.0), Some((4, 8)));
        assert_eq!(
            cell_at(120.0, 180.0),
            Some((1, 1)),
            "el borde pertenece a la celda siguiente"
        );
        assert_eq!(cell_at(39.0, 100.0), None);
        assert_eq!(cell_at(100.0, 79.0), None);
        assert_eq!(cell_at(760.0, 100.0), None);
        assert_eq!(cell_at(100.0, 580.0), None);
    }

    #[test]
    fn origen_de_celdas() {
        assert_eq!(cell_left(0), 40.0);
        assert_eq!(cell_left(8) + CELL_W, 760.0);
        assert_eq!(cell_top(4) + CELL_H, 580.0);
    }

    #[test]
    fn el_centro_de_la_pantalla_es_el_origen_del_mundo() {
        assert_eq!(screen_to_world(400.0, 300.0), (0.0, 0.0));
        assert_eq!(screen_to_world(0.0, 0.0), (-400.0, 300.0));
        assert_eq!(screen_to_world(800.0, 600.0), (400.0, -300.0));
    }
}
