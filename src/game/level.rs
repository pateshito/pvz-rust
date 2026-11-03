//! Un nivel: qué semillas hay, cuántos soles al empezar y cuándo llegan los zombies.

use super::defs::{PlantKind, ZombieKind};

#[derive(Clone, Debug)]
pub struct Wave {
    /// Segundos desde el inicio de la partida.
    pub time: f32,
    /// Cada zombie aparece en una fila al azar.
    pub zombies: Vec<ZombieKind>,
}

/// Niveles de la campaña de prueba.
pub const LEVEL_COUNT: usize = 6;

#[derive(Clone, Debug)]
pub struct Level {
    pub name: &'static str,
    pub start_sun: u32,
    /// Semillas disponibles, en el orden de la barra.
    pub seeds: Vec<PlantKind>,
    pub waves: Vec<Wave>,
    /// Segundos hasta el primer sol del cielo.
    pub first_sky_sun: f32,
    /// Rango (mín, máx) en segundos entre soles del cielo.
    pub sky_sun_interval: (f32, f32),
    pub sky_sun_value: u32,
}

impl Level {
    pub fn total_zombies(&self) -> usize {
        self.waves.iter().map(|w| w.zombies.len()).sum()
    }

    /// Nivel de prueba: girasol, los dos lanzaguisantes, nuez y papa mina contra
    /// zombies básicos. Los soles del cielo siguen siendo generosos; el balance
    /// del lanzaguisantes se comprueba con un jugador automático en `sim.rs`.
    pub fn test_level() -> Level {
        let wave = |time: f32, n: usize| Wave {
            time,
            zombies: vec![ZombieKind::Basic; n],
        };
        Level {
            name: "Día de prueba",
            start_sun: 250,
            seeds: vec![
                PlantKind::Sunflower,
                PlantKind::PeaShooter,
                PlantKind::SnowPea,
                PlantKind::Wallnut,
                PlantKind::PotatoMine,
            ],
            waves: vec![
                wave(12.0, 1),
                wave(35.0, 1),
                wave(55.0, 2),
                wave(80.0, 2),
                wave(105.0, 3),
                wave(130.0, 4),
            ],
            first_sky_sun: 4.0,
            sky_sun_interval: (4.0, 6.0),
            sky_sun_value: 25,
        }
    }

    /// Nivel de la campaña. Los seis comparten zombi básico; suben el número
    /// de oleadas, bajan los soles iniciales y añaden semillas de apoyo.
    pub fn campaign(index: usize) -> Level {
        let wave = |time: f32, n: usize| Wave {
            time,
            zombies: vec![ZombieKind::Basic; n],
        };
        match index.min(LEVEL_COUNT - 1) {
            0 => Level {
                name: "Día 1 · Primeros pasos",
                start_sun: 50,
                seeds: vec![PlantKind::Sunflower, PlantKind::PeaShooter],
                waves: vec![wave(20.0, 1), wave(45.0, 2), wave(75.0, 2)],
                first_sky_sun: 4.0,
                sky_sun_interval: (4.0, 6.0),
                sky_sun_value: 25,
            },
            1 => Level {
                name: "Día 2 · Nuez de guardia",
                start_sun: 50,
                seeds: vec![
                    PlantKind::Sunflower,
                    PlantKind::PeaShooter,
                    PlantKind::Wallnut,
                ],
                waves: vec![wave(18.0, 1), wave(40.0, 2), wave(65.0, 2), wave(95.0, 3)],
                first_sky_sun: 4.0,
                sky_sun_interval: (4.0, 6.0),
                sky_sun_value: 25,
            },
            2 => Level {
                name: "Día 3 · Hielo",
                start_sun: 50,
                seeds: vec![
                    PlantKind::Sunflower,
                    PlantKind::PeaShooter,
                    PlantKind::SnowPea,
                    PlantKind::Wallnut,
                ],
                waves: vec![
                    wave(18.0, 1),
                    wave(38.0, 2),
                    wave(60.0, 3),
                    wave(85.0, 3),
                    wave(110.0, 4),
                ],
                first_sky_sun: 4.0,
                sky_sun_interval: (4.0, 6.0),
                sky_sun_value: 25,
            },
            3 => Level {
                name: "Día 4 · Papa mina",
                start_sun: 50,
                seeds: vec![
                    PlantKind::Sunflower,
                    PlantKind::PeaShooter,
                    PlantKind::SnowPea,
                    PlantKind::Wallnut,
                    PlantKind::PotatoMine,
                ],
                waves: vec![
                    wave(16.0, 1),
                    wave(36.0, 2),
                    wave(58.0, 3),
                    wave(82.0, 3),
                    wave(106.0, 4),
                    wave(132.0, 4),
                ],
                first_sky_sun: 3.5,
                sky_sun_interval: (4.0, 6.0),
                sky_sun_value: 25,
            },
            _ => Level {
                name: "Día 6 · Última línea",
                start_sun: 50,
                seeds: vec![
                    PlantKind::Sunflower,
                    PlantKind::PeaShooter,
                    PlantKind::SnowPea,
                    PlantKind::Wallnut,
                    PlantKind::PotatoMine,
                ],
                waves: vec![
                    wave(12.0, 1),
                    wave(28.0, 2),
                    wave(46.0, 3),
                    wave(66.0, 3),
                    wave(86.0, 4),
                    wave(108.0, 4),
                    wave(130.0, 5),
                    wave(152.0, 5),
                    wave(176.0, 6),
                ],
                first_sky_sun: 3.0,
                sky_sun_interval: (3.0, 5.0),
                sky_sun_value: 25,
            },
        }
    }
}
