//! Un nivel: qué semillas hay, cuántos soles al empezar y cuándo llegan los zombies.

use super::defs::{PlantKind, ZombieKind};
use ZombieKind::{Basic, Buckethead, Conehead};

#[derive(Clone, Debug)]
pub struct Wave {
    /// Segundos desde el inicio de la partida.
    pub time: f32,
    /// Zombies de la oleada, en orden. La fila se elige al azar; el tipo, la
    /// cantidad y el momento son fijos.
    pub zombies: Vec<ZombieKind>,
}

impl Wave {
    /// Crea una oleada con cantidades exactas por tipo, en este orden.
    pub fn with_counts(time: f32, counts: &[(ZombieKind, usize)]) -> Self {
        let zombies = counts
            .iter()
            .flat_map(|&(kind, count)| std::iter::repeat(kind).take(count))
            .collect();
        Self { time, zombies }
    }
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
        let wave = |time: f32, n: usize| Wave::with_counts(time, &[(Basic, n)]);
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

    /// Nivel de la campaña. Cada oleada fija el momento y la cantidad exacta de
    /// cada tipo de zombie; solo la fila se elige al azar.
    pub fn campaign(index: usize) -> Level {
        let wave = |time: f32, counts: &[(ZombieKind, usize)]| Wave::with_counts(time, counts);
        match index.min(LEVEL_COUNT - 1) {
            0 => Level {
                name: "Día 1 · Primeros pasos",
                start_sun: 50,
                seeds: vec![PlantKind::Sunflower, PlantKind::PeaShooter],
                waves: vec![
                    wave(20.0, &[(Basic, 1)]),
                    wave(45.0, &[(Basic, 2)]),
                    wave(75.0, &[(Basic, 2)]),
                ],
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
                waves: vec![
                    wave(18.0, &[(Basic, 1)]),
                    wave(40.0, &[(Basic, 2)]),
                    wave(65.0, &[(Basic, 2)]),
                    wave(95.0, &[(Basic, 2), (Conehead, 1)]),
                ],
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
                    wave(18.0, &[(Basic, 1)]),
                    wave(38.0, &[(Basic, 2)]),
                    wave(60.0, &[(Basic, 2), (Conehead, 1)]),
                    wave(85.0, &[(Basic, 2), (Conehead, 1)]),
                    wave(110.0, &[(Basic, 2), (Conehead, 2)]),
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
                    wave(16.0, &[(Basic, 1)]),
                    wave(36.0, &[(Basic, 2)]),
                    wave(58.0, &[(Basic, 2), (Conehead, 1)]),
                    wave(82.0, &[(Basic, 2), (Conehead, 1)]),
                    wave(106.0, &[(Basic, 2), (Conehead, 1), (Buckethead, 1)]),
                    wave(132.0, &[(Basic, 3), (Conehead, 1)]),
                ],
                first_sky_sun: 3.5,
                sky_sun_interval: (4.0, 6.0),
                sky_sun_value: 25,
            },
            4 => Level {
                name: "Día 5 · Conos y cubetas",
                start_sun: 50,
                seeds: vec![
                    PlantKind::Sunflower,
                    PlantKind::PeaShooter,
                    PlantKind::SnowPea,
                    PlantKind::Wallnut,
                    PlantKind::PotatoMine,
                ],
                waves: vec![
                    wave(14.0, &[(Basic, 1)]),
                    wave(32.0, &[(Basic, 2)]),
                    wave(52.0, &[(Basic, 2), (Conehead, 1)]),
                    wave(74.0, &[(Basic, 2), (Conehead, 1)]),
                    wave(96.0, &[(Basic, 2), (Conehead, 1), (Buckethead, 1)]),
                    wave(120.0, &[(Basic, 3), (Conehead, 1), (Buckethead, 1)]),
                    wave(146.0, &[(Basic, 3), (Conehead, 2), (Buckethead, 1)]),
                ],
                first_sky_sun: 3.0,
                sky_sun_interval: (3.0, 5.0),
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
                    wave(12.0, &[(Basic, 1)]),
                    wave(28.0, &[(Basic, 2)]),
                    wave(46.0, &[(Basic, 2), (Conehead, 1)]),
                    wave(66.0, &[(Basic, 2), (Conehead, 1)]),
                    wave(86.0, &[(Basic, 3), (Conehead, 1)]),
                    wave(108.0, &[(Basic, 3), (Conehead, 1), (Buckethead, 1)]),
                    wave(130.0, &[(Basic, 3), (Conehead, 1), (Buckethead, 1)]),
                    wave(152.0, &[(Basic, 4), (Conehead, 1), (Buckethead, 1)]),
                    wave(176.0, &[(Basic, 4), (Conehead, 2), (Buckethead, 1)]),
                ],
                first_sky_sun: 3.0,
                sky_sun_interval: (3.0, 5.0),
                sky_sun_value: 25,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn una_oleada_guarda_las_cantidades_por_tipo() {
        let wave = Wave::with_counts(10.0, &[(Basic, 2), (Conehead, 1), (Buckethead, 1)]);
        assert_eq!(wave.time, 10.0);
        assert_eq!(
            wave.zombies,
            vec![
                ZombieKind::Basic,
                ZombieKind::Basic,
                ZombieKind::Conehead,
                ZombieKind::Buckethead
            ]
        );
    }

    #[test]
    fn los_niveles_fijan_horarios_y_tipos() {
        for index in 0..LEVEL_COUNT {
            let level = Level::campaign(index);
            let mut last = f32::NEG_INFINITY;
            for wave in &level.waves {
                assert!(wave.time > last, "oleadas desordenadas en el nivel {index}");
                last = wave.time;
                assert!(!wave.zombies.is_empty());
            }
            assert_eq!(
                level.total_zombies(),
                level
                    .waves
                    .iter()
                    .map(|wave| wave.zombies.len())
                    .sum::<usize>()
            );
        }
    }

    #[test]
    fn los_conos_y_las_cubetas_entran_por_nivel() {
        let basic_only = Level::campaign(0);
        assert!(basic_only
            .waves
            .iter()
            .flat_map(|wave| &wave.zombies)
            .all(|kind| *kind == Basic));

        let last = Level::campaign(LEVEL_COUNT - 1);
        let kinds: Vec<ZombieKind> = last
            .waves
            .iter()
            .flat_map(|wave| wave.zombies.iter().copied())
            .collect();
        assert!(kinds.contains(&Conehead));
        assert!(kinds.contains(&Buckethead));
    }
}
