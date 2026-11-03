//! Un nivel: qué semillas hay, cuántos soles al empezar y cuándo llegan los zombies.

use super::defs::{PlantKind, ZombieKind};

#[derive(Clone, Debug)]
pub struct Wave {
    /// Segundos desde el inicio de la partida.
    pub time: f32,
    /// Cada zombie aparece en una fila al azar.
    pub zombies: Vec<ZombieKind>,
}

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
}
