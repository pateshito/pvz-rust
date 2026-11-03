//! Simulación de la partida, sin nada de Bevy: así se puede probar entera.
//!
//! Bevy solo (1) manda órdenes (`plant`, `collect_sun`), (2) llama a `step(dt)`,
//! (3) lee el estado para dibujarlo y (4) recoge los `SimEvent` para crear y
//! destruir las entidades. Coordenadas: las de pantalla de `board.rs`.

use super::board::*;
use super::defs::*;
use super::level::Level;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

pub type Id = u32;

/// Del borde izquierdo de la celda al borde izquierdo del guisante al salir.
pub const PEA_START_DX: f32 = 58.0;
/// Del borde superior de la celda al centro vertical del guisante.
pub const PEA_Y_OFFSET: f32 = 36.0;
pub const PEA_WIDTH: f32 = 28.0;

pub const SUN_FALL_SPEED: f32 = 60.0;
/// Segundos que dura un sol en el suelo antes de desaparecer.
pub const SUN_LIFETIME: f32 = 8.0;
pub const SUN_PICK_RADIUS: f32 = 40.0;

/// Origen x de un zombie recién nacido (justo fuera de la pantalla).
pub const ZOMBIE_SPAWN_X: f32 = SCREEN_W + 10.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Playing,
    Won,
    Lost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlantError {
    GameOver,
    NoSuchSeed,
    OutOfBoard,
    CellOccupied,
    OnCooldown,
    NotEnoughSun,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeedStatus {
    Ready,
    Cooldown,
    NoSun,
}

#[derive(Clone, Copy, Debug)]
pub struct Seed {
    pub kind: PlantKind,
    pub cooldown_left: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Plant {
    pub id: Id,
    pub kind: PlantKind,
    pub row: usize,
    pub col: usize,
    pub hp: f32,
    /// Avance (0..1) del ciclo de disparo; `None` si no hay a quién disparar.
    pub shoot_phase: Option<f32>,
    /// Girasol: segundos hasta el próximo sol. Papa mina: segundos bajo tierra.
    pub timer: f32,
    /// La papa mina ya puede reventar.
    pub armed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ZombieStatus {
    Walking,
    /// Comiéndose a la planta con ese id.
    Eating(Id),
    Dying {
        elapsed: f32,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Zombie {
    pub id: Id,
    pub kind: ZombieKind,
    pub row: usize,
    /// Origen del personaje (borde izquierdo de su animación).
    pub x: f32,
    pub hp: f32,
    /// Variación individual de velocidad; la animación la usa también.
    pub speed_mul: f32,
    /// Segundos que le quedan de hielo (media velocidad).
    pub chill: f32,
    pub status: ZombieStatus,
}

impl Zombie {
    pub fn is_alive(&self) -> bool {
        self.hp > 0.0 && !matches!(self.status, ZombieStatus::Dying { .. })
    }

    /// Multiplicador de avance: la variación propia y, si está helado, la mitad.
    pub fn pace(&self) -> f32 {
        let slow = if self.chill > 0.0 { 0.5 } else { 1.0 };
        self.speed_mul * slow
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Pea {
    pub id: Id,
    pub row: usize,
    /// Borde izquierdo del guisante.
    pub x: f32,
    pub damage: f32,
    pub speed: f32,
    /// Segundos de hielo que aplica al impactar. 0 es un guisante normal.
    pub chill_for: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Sun {
    pub id: Id,
    /// Centro del sol.
    pub x: f32,
    pub y: f32,
    pub target_y: f32,
    pub value: u32,
    pub landed_for: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SimEvent {
    PlantPlaced(Id),
    PlantDied(Id),
    ZombieSpawned(Id),
    /// Empieza su animación de muerte (ya no recibe daño ni muerde).
    ZombieDied(Id),
    /// Terminó la animación de muerte: hay que quitarlo.
    ZombieRemoved(Id),
    PeaFired(Id),
    PeaRemoved {
        id: Id,
        hit: bool,
    },
    SunSpawned(Id),
    SunCollected(Id),
    SunExpired(Id),
    Won,
    Lost,
}

pub struct Sim {
    pub level: Level,
    pub time: f32,
    pub sun: u32,
    pub outcome: Outcome,
    pub seeds: Vec<Seed>,
    pub plants: Vec<Plant>,
    pub zombies: Vec<Zombie>,
    pub peas: Vec<Pea>,
    pub suns: Vec<Sun>,
    /// Zombies que ya salieron / que ya murieron.
    pub spawned: usize,
    pub killed: usize,
    grid: [[Option<Id>; COLS]; ROWS],
    next_wave: usize,
    sky_timer: f32,
    next_id: Id,
    rng: StdRng,
    events: Vec<SimEvent>,
}

impl Sim {
    pub fn new(level: Level, rng_seed: u64) -> Self {
        let seeds = level
            .seeds
            .iter()
            .map(|&kind| Seed {
                kind,
                cooldown_left: 0.0,
            })
            .collect();
        Self {
            time: 0.0,
            sun: level.start_sun,
            outcome: Outcome::Playing,
            seeds,
            plants: Vec::new(),
            zombies: Vec::new(),
            peas: Vec::new(),
            suns: Vec::new(),
            spawned: 0,
            killed: 0,
            grid: [[None; COLS]; ROWS],
            next_wave: 0,
            sky_timer: level.first_sky_sun,
            next_id: 1,
            rng: StdRng::seed_from_u64(rng_seed),
            events: Vec::new(),
            level,
        }
    }

    pub fn take_events(&mut self) -> Vec<SimEvent> {
        std::mem::take(&mut self.events)
    }

    fn alloc_id(&mut self) -> Id {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    // ---------- Órdenes del jugador ----------

    pub fn seed_status(&self, index: usize) -> SeedStatus {
        let Some(seed) = self.seeds.get(index) else {
            return SeedStatus::Cooldown;
        };
        if seed.cooldown_left > 0.0 {
            SeedStatus::Cooldown
        } else if self.sun < plant_def(seed.kind).cost {
            SeedStatus::NoSun
        } else {
            SeedStatus::Ready
        }
    }

    /// Fracción (0..1) del enfriamiento que falta.
    pub fn seed_cooldown_frac(&self, index: usize) -> f32 {
        self.seeds.get(index).map_or(0.0, |s| {
            (s.cooldown_left / plant_def(s.kind).cooldown).clamp(0.0, 1.0)
        })
    }

    pub fn plant(&mut self, kind: PlantKind, row: usize, col: usize) -> Result<Id, PlantError> {
        if self.outcome != Outcome::Playing {
            return Err(PlantError::GameOver);
        }
        let seed = self
            .seeds
            .iter()
            .position(|s| s.kind == kind)
            .ok_or(PlantError::NoSuchSeed)?;
        if row >= ROWS || col >= COLS {
            return Err(PlantError::OutOfBoard);
        }
        if self.grid[row][col].is_some() {
            return Err(PlantError::CellOccupied);
        }
        let def = plant_def(kind);
        if self.seeds[seed].cooldown_left > 0.0 {
            return Err(PlantError::OnCooldown);
        }
        if self.sun < def.cost {
            return Err(PlantError::NotEnoughSun);
        }

        self.sun -= def.cost;
        self.seeds[seed].cooldown_left = def.cooldown;
        let id = self.alloc_id();
        self.plants.push(Plant {
            id,
            kind,
            row,
            col,
            hp: def.max_hp,
            shoot_phase: None,
            timer: def.producer.map(|p| p.first).unwrap_or(0.0),
            armed: false,
        });
        self.grid[row][col] = Some(id);
        self.events.push(SimEvent::PlantPlaced(id));
        Ok(id)
    }

    pub fn plant_at(&self, row: usize, col: usize) -> Option<&Plant> {
        let id = (*self.grid.get(row)?.get(col)?)?;
        self.plants.iter().find(|p| p.id == id)
    }

    /// Sol bajo un punto de pantalla (el de más arriba si se solapan).
    pub fn sun_at(&self, x: f32, y: f32) -> Option<Id> {
        self.suns
            .iter()
            .rev()
            .find(|s| (s.x - x).powi(2) + (s.y - y).powi(2) <= SUN_PICK_RADIUS.powi(2))
            .map(|s| s.id)
    }

    pub fn collect_sun(&mut self, id: Id) -> bool {
        let Some(pos) = self.suns.iter().position(|s| s.id == id) else {
            return false;
        };
        let sun = self.suns.remove(pos);
        self.sun += sun.value;
        self.events.push(SimEvent::SunCollected(id));
        true
    }

    // ---------- Avance del tiempo ----------

    pub fn step(&mut self, dt: f32) {
        if self.outcome != Outcome::Playing {
            return;
        }
        self.time += dt;
        for seed in &mut self.seeds {
            seed.cooldown_left = (seed.cooldown_left - dt).max(0.0);
        }
        self.update_sky_suns(dt);
        self.update_suns(dt);
        self.update_waves();
        self.update_plants(dt);
        self.update_peas(dt);
        self.update_zombies(dt);
        self.check_outcome();
    }

    fn update_sky_suns(&mut self, dt: f32) {
        self.sky_timer -= dt;
        if self.sky_timer > 0.0 {
            return;
        }
        let (lo, hi) = self.level.sky_sun_interval;
        self.sky_timer = if hi > lo {
            self.rng.gen_range(lo..hi)
        } else {
            lo
        };

        let id = self.alloc_id();
        let sun = Sun {
            id,
            x: self.rng.gen_range(100.0..700.0),
            y: -40.0,
            target_y: self.rng.gen_range(180.0..520.0),
            value: self.level.sky_sun_value,
            landed_for: 0.0,
        };
        self.suns.push(sun);
        self.events.push(SimEvent::SunSpawned(id));
    }

    fn update_suns(&mut self, dt: f32) {
        let mut expired = Vec::new();
        for sun in &mut self.suns {
            if sun.y < sun.target_y {
                sun.y = (sun.y + SUN_FALL_SPEED * dt).min(sun.target_y);
            } else {
                sun.landed_for += dt;
                if sun.landed_for >= SUN_LIFETIME {
                    expired.push(sun.id);
                }
            }
        }
        self.suns.retain(|s| !expired.contains(&s.id));
        self.events
            .extend(expired.into_iter().map(SimEvent::SunExpired));
    }

    fn update_waves(&mut self) {
        while let Some(wave) = self.level.waves.get(self.next_wave) {
            if self.time < wave.time {
                break;
            }
            let kinds = wave.zombies.clone();
            self.next_wave += 1;
            for kind in kinds {
                self.spawn_zombie(kind);
            }
        }
    }

    fn spawn_zombie(&mut self, kind: ZombieKind) {
        let def = zombie_def(kind);
        let id = self.alloc_id();
        let zombie = Zombie {
            id,
            kind,
            row: self.rng.gen_range(0..ROWS),
            x: ZOMBIE_SPAWN_X + self.rng.gen_range(0.0..40.0),
            hp: def.max_hp,
            speed_mul: self.rng.gen_range(0.9..1.1),
            chill: 0.0,
            status: ZombieStatus::Walking,
        };
        self.zombies.push(zombie);
        self.spawned += 1;
        self.events.push(SimEvent::ZombieSpawned(id));
    }

    fn update_plants(&mut self, dt: f32) {
        // (fila, x, daño, velocidad, segundos de hielo)
        let mut shots: Vec<(usize, f32, f32, f32, f32)> = Vec::new();
        let mut produced: Vec<(usize, usize, u32)> = Vec::new();

        for i in 0..self.plants.len() {
            let Plant { kind, row, col, .. } = self.plants[i];
            let def = plant_def(kind);

            if let Some(shooter) = def.shooter {
                let plant_center = cell_left(col) + CELL_W * 0.5;
                let has_target = self.zombies.iter().any(|z| {
                    let zd = zombie_def(z.kind);
                    z.row == row
                        && z.is_alive()
                        && z.x + zd.hit_left < SCREEN_W
                        && z.x + zd.hit_right > plant_center
                });

                let plant = &mut self.plants[i];
                plant.shoot_phase = match (has_target, plant.shoot_phase) {
                    (false, _) => None,
                    (true, None) => Some(0.0),
                    (true, Some(p)) => {
                        let next = p + dt / shooter.interval;
                        if p < shooter.fire_phase && next >= shooter.fire_phase {
                            shots.push((
                                row,
                                cell_left(col) + PEA_START_DX,
                                shooter.damage,
                                shooter.pea_speed,
                                shooter.chill,
                            ));
                        }
                        Some(if next >= 1.0 { next - 1.0 } else { next })
                    }
                };
            }

            if let Some(producer) = def.producer {
                let plant = &mut self.plants[i];
                plant.timer -= dt;
                if plant.timer <= 0.0 {
                    plant.timer = producer.interval;
                    produced.push((row, col, producer.value));
                }
            }

            if let Some(mine) = def.mine {
                let plant = &mut self.plants[i];
                if !plant.armed {
                    plant.timer += dt;
                    if plant.timer >= mine.arm_time {
                        plant.armed = true;
                    }
                }
            }
        }

        for (row, x, damage, speed, chill_for) in shots {
            let id = self.alloc_id();
            self.peas.push(Pea {
                id,
                row,
                x,
                damage,
                speed,
                chill_for,
            });
            self.events.push(SimEvent::PeaFired(id));
        }

        for (row, col, value) in produced {
            let id = self.alloc_id();
            self.suns.push(Sun {
                id,
                x: cell_left(col) + CELL_W * 0.5,
                y: cell_top(row) + 28.0,
                target_y: cell_top(row) + 28.0,
                value,
                landed_for: 0.0,
            });
            self.events.push(SimEvent::SunSpawned(id));
        }

        // La papa armada revienta al zombie que la pisa y desaparece.
        let blasts: Vec<(Id, Id, f32)> = self
            .plants
            .iter()
            .filter(|plant| plant.armed && plant_def(plant.kind).mine.is_some())
            .filter_map(|plant| {
                let damage = plant_def(plant.kind).mine?.damage;
                let zombie = self.zombie_on_cell(plant.row, plant.col)?;
                Some((plant.id, self.zombies[zombie].id, damage))
            })
            .collect();
        for (plant_id, zombie_id, damage) in blasts {
            if let Some(zombie) = self.zombies.iter_mut().find(|z| z.id == zombie_id) {
                zombie.hp -= damage;
            }
            if let Some(index) = self.plants.iter().position(|p| p.id == plant_id) {
                let plant = self.plants.remove(index);
                self.grid[plant.row][plant.col] = None;
                self.events.push(SimEvent::PlantDied(plant.id));
            }
        }
    }

    /// Zombie vivo que está pisando esa celda, el que va más adelante.
    fn zombie_on_cell(&self, row: usize, col: usize) -> Option<usize> {
        let left = cell_left(col);
        let right = left + CELL_W;
        let mut best: Option<usize> = None;
        let mut best_x = f32::MAX;
        for (index, zombie) in self.zombies.iter().enumerate() {
            if zombie.row != row || !zombie.is_alive() {
                continue;
            }
            let front = zombie.x + zombie_def(zombie.kind).bite_reach;
            if front > left && zombie.x < right && zombie.x < best_x {
                best = Some(index);
                best_x = zombie.x;
            }
        }
        best
    }

    fn update_peas(&mut self, dt: f32) {
        let mut i = 0;
        while i < self.peas.len() {
            let pea = self.peas[i];
            let new_x = pea.x + pea.speed * dt;

            // El primer zombie vivo que el guisante toca en este avance.
            let mut hit: Option<usize> = None;
            let mut hit_x = f32::MAX;
            for (zi, z) in self.zombies.iter().enumerate() {
                if z.row != pea.row || !z.is_alive() {
                    continue;
                }
                let zd = zombie_def(z.kind);
                let (left, right) = (z.x + zd.hit_left, z.x + zd.hit_right);
                if left >= SCREEN_W {
                    continue;
                }
                if new_x + PEA_WIDTH >= left && pea.x <= right && z.x < hit_x {
                    hit = Some(zi);
                    hit_x = z.x;
                }
            }

            if let Some(zi) = hit {
                let zombie = &mut self.zombies[zi];
                zombie.hp -= pea.damage;
                if pea.chill_for > 0.0 {
                    zombie.chill = zombie.chill.max(pea.chill_for);
                }
                self.peas.remove(i);
                self.events.push(SimEvent::PeaRemoved {
                    id: pea.id,
                    hit: true,
                });
            } else if new_x > SCREEN_W + 30.0 {
                self.peas.remove(i);
                self.events.push(SimEvent::PeaRemoved {
                    id: pea.id,
                    hit: false,
                });
            } else {
                self.peas[i].x = new_x;
                i += 1;
            }
        }
    }

    /// Planta de la fila del zombie que está al alcance de sus manos.
    fn blocking_plant(&self, z: &Zombie) -> Option<Id> {
        let front = z.x + zombie_def(z.kind).bite_reach;
        self.plants
            .iter()
            .filter(|p| p.row == z.row)
            .find(|p| {
                // La papa, armada o no, no se come: se pisa y luego revienta.
                if plant_def(p.kind).mine.is_some() {
                    return false;
                }
                let left = cell_left(p.col);
                front > left && front <= left + CELL_W - 10.0
            })
            .map(|p| p.id)
    }

    fn update_zombies(&mut self, dt: f32) {
        for zombie in &mut self.zombies {
            zombie.chill = (zombie.chill - dt).max(0.0);
        }
        let mut i = 0;
        while i < self.zombies.len() {
            let z = self.zombies[i];
            let def = zombie_def(z.kind);

            if let ZombieStatus::Dying { elapsed } = z.status {
                let elapsed = elapsed + dt;
                if elapsed >= def.death_time {
                    self.zombies.remove(i);
                    self.events.push(SimEvent::ZombieRemoved(z.id));
                } else {
                    self.zombies[i].status = ZombieStatus::Dying { elapsed };
                    i += 1;
                }
                continue;
            }

            if z.hp <= 0.0 {
                self.zombies[i].status = ZombieStatus::Dying { elapsed: 0.0 };
                self.killed += 1;
                self.events.push(SimEvent::ZombieDied(z.id));
                i += 1;
                continue;
            }

            match z.status {
                ZombieStatus::Walking => match self.blocking_plant(&z) {
                    Some(plant_id) => self.zombies[i].status = ZombieStatus::Eating(plant_id),
                    None => self.zombies[i].x -= def.speed * z.pace() * dt,
                },
                ZombieStatus::Eating(plant_id) => {
                    match self.plants.iter().position(|p| p.id == plant_id) {
                        Some(pi) => {
                            self.plants[pi].hp -= def.bite_dps * dt;
                            if self.plants[pi].hp <= 0.0 {
                                let plant = self.plants.remove(pi);
                                self.grid[plant.row][plant.col] = None;
                                self.events.push(SimEvent::PlantDied(plant.id));
                                self.zombies[i].status = ZombieStatus::Walking;
                            }
                        }
                        // Otro zombie ya se la comió.
                        None => self.zombies[i].status = ZombieStatus::Walking,
                    }
                }
                ZombieStatus::Dying { .. } => unreachable!(),
            }
            i += 1;
        }
    }

    fn check_outcome(&mut self) {
        let lost = self
            .zombies
            .iter()
            .any(|z| z.is_alive() && z.x + zombie_def(z.kind).bite_reach < LOSE_X);
        if lost {
            self.outcome = Outcome::Lost;
            self.events.push(SimEvent::Lost);
            return;
        }
        let all_out = self.next_wave >= self.level.waves.len();
        if all_out && self.zombies.iter().all(|z| !z.is_alive()) {
            self.outcome = Outcome::Won;
            self.events.push(SimEvent::Won);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::level::Wave;

    const DT: f32 = 1.0 / 60.0;

    /// Nivel sin zombies reales: su unica oleada llega tan tarde que la partida
    /// nunca termina durante la prueba.
    fn empty_level() -> Level {
        Level {
            name: "test",
            start_sun: 500,
            seeds: vec![PlantKind::PeaShooter],
            waves: vec![Wave {
                time: 1.0e9,
                zombies: vec![ZombieKind::Basic],
            }],
            first_sky_sun: 1000.0,
            sky_sun_interval: (1000.0, 1000.0),
            sky_sun_value: 25,
        }
    }

    fn level_with_zombie_at(time: f32) -> Level {
        Level {
            waves: vec![Wave {
                time,
                zombies: vec![ZombieKind::Basic],
            }],
            ..empty_level()
        }
    }

    fn run(sim: &mut Sim, seconds: f32) {
        for _ in 0..(seconds / DT) as usize {
            sim.step(DT);
        }
    }

    /// Un zombie sin movimiento aleatorio en la fila indicada y a la x indicada.
    fn place_zombie(sim: &mut Sim, row: usize, x: f32) -> Id {
        let id = sim.alloc_id();
        sim.zombies.push(Zombie {
            id,
            kind: ZombieKind::Basic,
            row,
            x,
            hp: zombie_def(ZombieKind::Basic).max_hp,
            speed_mul: 1.0,
            chill: 0.0,
            status: ZombieStatus::Walking,
        });
        sim.spawned += 1;
        id
    }

    #[test]
    fn plantar_cuesta_soles_y_activa_el_enfriamiento() {
        let mut sim = Sim::new(empty_level(), 1);
        assert_eq!(sim.seed_status(0), SeedStatus::Ready);
        let id = sim.plant(PlantKind::PeaShooter, 2, 3).unwrap();
        assert_eq!(sim.sun, 400);
        assert_eq!(sim.plant_at(2, 3).map(|p| p.id), Some(id));
        assert_eq!(sim.seed_status(0), SeedStatus::Cooldown);
        assert_eq!(sim.seed_cooldown_frac(0), 1.0);
        assert_eq!(sim.take_events(), vec![SimEvent::PlantPlaced(id)]);

        run(&mut sim, 7.6);
        assert_eq!(sim.seed_status(0), SeedStatus::Ready);
    }

    #[test]
    fn plantar_rechaza_los_casos_invalidos() {
        let mut sim = Sim::new(empty_level(), 1);
        sim.plant(PlantKind::PeaShooter, 0, 0).unwrap();
        assert_eq!(
            sim.plant(PlantKind::PeaShooter, 1, 1),
            Err(PlantError::OnCooldown)
        );
        run(&mut sim, 8.0);
        assert_eq!(
            sim.plant(PlantKind::PeaShooter, 0, 0),
            Err(PlantError::CellOccupied)
        );
        assert_eq!(
            sim.plant(PlantKind::PeaShooter, 5, 0),
            Err(PlantError::OutOfBoard)
        );
        assert_eq!(
            sim.plant(PlantKind::PeaShooter, 0, 9),
            Err(PlantError::OutOfBoard)
        );
        sim.sun = 99;
        assert_eq!(
            sim.plant(PlantKind::PeaShooter, 1, 1),
            Err(PlantError::NotEnoughSun)
        );
        assert_eq!(sim.seed_status(0), SeedStatus::NoSun);
        sim.sun = 100;
        assert!(sim.plant(PlantKind::PeaShooter, 1, 1).is_ok());
        assert_eq!(sim.sun, 0);
    }

    #[test]
    fn el_lanzaguisantes_solo_dispara_si_hay_zombie_en_su_fila() {
        let mut sim = Sim::new(empty_level(), 1);
        sim.plant(PlantKind::PeaShooter, 2, 1).unwrap();
        place_zombie(&mut sim, 3, 600.0); // otra fila
        run(&mut sim, 3.0);
        assert!(sim.peas.is_empty());
        assert_eq!(sim.plants[0].shoot_phase, None);

        place_zombie(&mut sim, 2, 700.0);
        run(&mut sim, 1.0);
        assert!(sim.plants[0].shoot_phase.is_some());
        assert!(!sim.peas.is_empty(), "ya disparó");
        assert!(sim.peas.iter().all(|p| p.row == 2));
    }

    #[test]
    fn disparos_con_el_intervalo_de_la_planta() {
        let mut sim = Sim::new(empty_level(), 1);
        sim.plant(PlantKind::PeaShooter, 0, 0).unwrap();
        let z = place_zombie(&mut sim, 0, 700.0);
        sim.zombies.iter_mut().find(|s| s.id == z).unwrap().hp = 1.0e9; // aguanta
        sim.take_events();
        let mut fired = Vec::new();
        for _ in 0..(6.0 / DT) as usize {
            sim.step(DT);
            fired.extend(
                sim.take_events()
                    .into_iter()
                    .filter(|e| matches!(e, SimEvent::PeaFired(_)))
                    .map(|_| sim.time),
            );
        }
        assert!(fired.len() >= 4, "{fired:?}");
        let first = fired[0];
        assert!(
            (first - 1.425 * 0.5).abs() < 0.05,
            "primer disparo a mitad de ciclo: {first}"
        );
        for pair in fired.windows(2) {
            assert!((pair[1] - pair[0] - 1.425).abs() < 0.05, "{fired:?}");
        }
    }

    #[test]
    fn un_zombie_muere_a_catorce_guisantes() {
        let mut sim = Sim::new(empty_level(), 1);
        sim.plant(PlantKind::PeaShooter, 1, 0).unwrap();
        let z = place_zombie(&mut sim, 1, 500.0);
        sim.zombies[0].speed_mul = 0.0001; // casi quieto
        let mut hits = 0;
        for _ in 0..(40.0 / DT) as usize {
            sim.step(DT);
            for e in sim.take_events() {
                if matches!(e, SimEvent::PeaRemoved { hit: true, .. }) {
                    hits += 1;
                }
            }
            if sim.zombies.iter().any(|s| s.id == z && !s.is_alive()) {
                break;
            }
        }
        assert_eq!(hits, 14, "270 de vida / 20 de daño");
        assert_eq!(sim.killed, 1);
    }

    #[test]
    fn el_zombie_muerto_se_quita_tras_su_animacion() {
        let mut sim = Sim::new(empty_level(), 1);
        let z = place_zombie(&mut sim, 0, 500.0);
        sim.zombies[0].hp = 0.0;
        sim.step(DT);
        assert!(sim.take_events().contains(&SimEvent::ZombieDied(z)));
        assert!(!sim.zombies[0].is_alive());
        run(&mut sim, 3.0);
        assert_eq!(
            sim.zombies.len(),
            1,
            "sigue en el suelo mientras dura la animacion"
        );
        run(&mut sim, 0.5);
        assert!(sim.zombies.is_empty());
        assert!(sim.take_events().contains(&SimEvent::ZombieRemoved(z)));
    }

    #[test]
    fn el_zombie_se_come_la_planta_y_sigue_caminando() {
        let mut sim = Sim::new(empty_level(), 1);
        sim.plant(PlantKind::PeaShooter, 0, 2).unwrap();
        sim.take_events();
        let plant_id = sim.plants[0].id;
        // La planta ocupa la celda de x = 200..280; el zombie llega desde la derecha
        // y empieza a comer cuando su frente (x + 20) baja de 270.
        place_zombie(&mut sim, 0, 262.0);
        run(&mut sim, 1.5);
        assert_eq!(sim.zombies[0].status, ZombieStatus::Eating(plant_id));
        let eating_x = sim.zombies[0].x;

        // 300 de vida a 100 por segundo: ~3 s de mordiscos.
        run(&mut sim, 2.0);
        assert!(sim.plants.len() == 1, "aun no se la termina");
        run(&mut sim, 1.5);
        assert!(sim.plants.is_empty());
        assert!(sim.plant_at(0, 2).is_none(), "la celda queda libre");
        assert!(sim.take_events().contains(&SimEvent::PlantDied(plant_id)));

        let before = sim.zombies[0].x;
        run(&mut sim, 1.0);
        assert_eq!(sim.zombies[0].status, ZombieStatus::Walking);
        assert!(sim.zombies[0].x < before, "vuelve a caminar");
        assert!(eating_x < 262.0);
    }

    #[test]
    fn dos_zombies_comen_la_misma_planta_y_ambos_siguen() {
        let mut sim = Sim::new(empty_level(), 1);
        sim.plant(PlantKind::PeaShooter, 0, 2).unwrap();
        place_zombie(&mut sim, 0, 262.0);
        place_zombie(&mut sim, 0, 264.0);
        run(&mut sim, 3.0); // el doble de dps: la planta cae antes de 3 s
        assert!(sim.plants.is_empty());
        run(&mut sim, 0.2);
        assert!(sim
            .zombies
            .iter()
            .all(|z| z.status == ZombieStatus::Walking));
    }

    #[test]
    fn plantas_a_la_espalda_del_zombie_no_lo_frenan() {
        let mut sim = Sim::new(empty_level(), 1);
        place_zombie(&mut sim, 0, 100.0); // frente en x = 120
        sim.plant(PlantKind::PeaShooter, 0, 6).unwrap(); // celda de x = 520..600
        run(&mut sim, 2.0);
        assert_eq!(sim.zombies[0].status, ZombieStatus::Walking);
    }

    #[test]
    fn zombie_que_llega_a_la_casa_pierde_la_partida() {
        let mut sim = Sim::new(empty_level(), 1);
        place_zombie(&mut sim, 4, 30.0);
        run(&mut sim, 5.0);
        assert_eq!(sim.outcome, Outcome::Lost);
        assert!(sim.take_events().contains(&SimEvent::Lost));
        assert_eq!(
            sim.plant(PlantKind::PeaShooter, 0, 0),
            Err(PlantError::GameOver)
        );
        let t = sim.time;
        sim.step(DT);
        assert_eq!(sim.time, t, "el tiempo se congela");
    }

    #[test]
    fn gana_al_matar_a_todos_los_zombies_de_todas_las_oleadas() {
        let mut sim = Sim::new(level_with_zombie_at(2.0), 3);
        run(&mut sim, 1.0);
        assert_eq!(sim.outcome, Outcome::Playing, "falta la oleada");
        assert_eq!(sim.spawned, 0);
        run(&mut sim, 1.5);
        assert_eq!(sim.spawned, 1);
        assert_eq!(sim.outcome, Outcome::Playing);
        sim.zombies[0].hp = 0.0;
        run(&mut sim, 0.1);
        assert_eq!(sim.outcome, Outcome::Won);
        assert!(sim.take_events().contains(&SimEvent::Won));
    }

    #[test]
    fn los_soles_caen_se_recogen_y_caducan() {
        let mut level = empty_level();
        level.first_sky_sun = 1.0;
        level.sky_sun_interval = (50.0, 50.0);
        let mut sim = Sim::new(level, 5);
        run(&mut sim, 1.1);
        assert_eq!(sim.suns.len(), 1);
        let (id, x) = (sim.suns[0].id, sim.suns[0].x);
        assert_eq!(sim.sun_at(x, sim.suns[0].y), Some(id));
        assert_eq!(sim.sun_at(x + 200.0, sim.suns[0].y), None);
        let y0 = sim.suns[0].y;
        run(&mut sim, 1.0);
        assert!(sim.suns[0].y > y0, "cae");

        let before = sim.sun;
        assert!(sim.collect_sun(id));
        assert_eq!(sim.sun, before + 25);
        assert!(!sim.collect_sun(id), "no se recoge dos veces");
        assert!(sim.take_events().contains(&SimEvent::SunCollected(id)));

        // Un segundo sol que nadie recoge desaparece tras caer y esperar.
        sim.level.first_sky_sun = 0.0;
        sim.sky_timer = 0.0;
        sim.step(DT);
        let second = sim.suns[0].id;
        run(&mut sim, 25.0);
        assert!(sim.suns.iter().all(|s| s.id != second));
        assert!(sim.take_events().contains(&SimEvent::SunExpired(second)));
    }

    fn level_with(kind: PlantKind) -> Level {
        Level {
            seeds: vec![kind],
            ..empty_level()
        }
    }

    #[test]
    fn el_girasol_suelta_un_sol_y_luego_espera() {
        let mut sim = Sim::new(level_with(PlantKind::Sunflower), 1);
        sim.plant(PlantKind::Sunflower, 2, 2).unwrap();
        run(&mut sim, 7.4);
        assert!(sim.suns.is_empty(), "el primero tarda 7,5 s");
        run(&mut sim, 0.3);
        assert_eq!(sim.suns.len(), 1);
        assert_eq!(sim.suns[0].value, 25);
        let id = sim.suns[0].id;
        run(&mut sim, 5.0);
        assert!(
            sim.suns.iter().any(|s| s.id == id),
            "sigue en el césped antes de caducar"
        );
        assert_eq!(sim.suns.len(), 1, "el siguiente sale a los 24 s");
    }

    #[test]
    fn el_guisante_de_hielo_ralentiza_al_zombie() {
        let mut sim = Sim::new(level_with(PlantKind::SnowPea), 1);
        sim.plant(PlantKind::SnowPea, 0, 0).unwrap();
        place_zombie(&mut sim, 0, 500.0);
        sim.zombies[0].hp = 1.0e9;
        run(&mut sim, 3.0);
        assert!(sim.zombies[0].chill > 0.0, "el impacto deja hielo");
        let x = sim.zombies[0].x;
        run(&mut sim, 1.0);
        let moved = x - sim.zombies[0].x;
        assert!(
            (4.0..12.0).contains(&moved),
            "a media velocidad anda unos 9 px/s, anduvo {moved}"
        );
    }

    #[test]
    fn la_nuez_aguanta_mas_que_un_lanzaguisantes() {
        let mut sim = Sim::new(level_with(PlantKind::Wallnut), 1);
        sim.plant(PlantKind::Wallnut, 0, 2).unwrap();
        place_zombie(&mut sim, 0, 262.0);
        run(&mut sim, 6.0);
        assert_eq!(sim.plants.len(), 1, "4000 de vida a 100 por segundo");
        assert!(sim.plants[0].hp < 4000.0);
        assert!(sim.plants[0].hp > 3000.0, "{}", sim.plants[0].hp);
    }

    #[test]
    fn la_papa_se_pisa_y_al_armarse_revienta_al_zombie() {
        let mut sim = Sim::new(level_with(PlantKind::PotatoMine), 1);
        let plant = sim.plant(PlantKind::PotatoMine, 0, 2).unwrap();
        place_zombie(&mut sim, 0, 250.0);
        sim.zombies[0].speed_mul = 0.0;
        run(&mut sim, 14.0);
        assert!(!sim.plants[0].armed);
        assert_eq!(sim.zombies[0].status, ZombieStatus::Walking);
        assert_eq!(sim.plants[0].id, plant);
        run(&mut sim, 1.2);
        assert!(sim.plants.is_empty(), "la mina desaparece al explotar");
        assert!(sim.take_events().contains(&SimEvent::PlantDied(plant)));
        assert!(sim.zombies[0].hp <= 0.0);
        assert!(!sim.zombies[0].is_alive());
    }

    #[test]
    fn la_misma_semilla_da_la_misma_partida() {
        let play = |seed| {
            let mut sim = Sim::new(Level::test_level(), seed);
            run(&mut sim, 60.0);
            (
                sim.zombies
                    .iter()
                    .map(|z| (z.row, z.x.to_bits()))
                    .collect::<Vec<_>>(),
                sim.suns
                    .iter()
                    .map(|s| (s.x.to_bits(), s.target_y.to_bits()))
                    .collect::<Vec<_>>(),
            )
        };
        assert_eq!(play(7), play(7));
        assert_ne!(play(7), play(8));
    }

    /// Jugador automático: recoge todos los soles y planta en cuanto puede pagarlo.
    /// Primero refuerza las filas donde ya hay zombies (como haría una persona al
    /// verlos entrar), y si no hay urgencia sigue la lista `plan`.
    fn bot(level: Level, seed: u64, plan: &[(usize, usize)]) -> (Outcome, Sim) {
        let mut sim = Sim::new(level, seed);
        for _ in 0..(600.0 / DT) as usize {
            let suns: Vec<Id> = sim.suns.iter().map(|s| s.id).collect();
            for id in suns {
                sim.collect_sun(id);
            }

            // Filas con zombies vivos, la más amenazada (zombie más adelantado) primero.
            let mut threatened: Vec<(f32, usize)> = sim
                .zombies
                .iter()
                .filter(|z| z.is_alive())
                .map(|z| (z.x, z.row))
                .collect();
            threatened.sort_by(|a, b| a.0.total_cmp(&b.0));

            let mut wanted: Vec<(usize, usize)> = Vec::new();
            for &(_, row) in &threatened {
                let zombies_in_row = sim
                    .zombies
                    .iter()
                    .filter(|z| z.is_alive() && z.row == row)
                    .count();
                let plants_in_row = (0..COLS)
                    .filter(|&c| sim.plant_at(row, c).is_some())
                    .count();
                if plants_in_row < zombies_in_row.min(3) {
                    if let Some(col) = [1usize, 2, 3]
                        .into_iter()
                        .find(|&c| sim.plant_at(row, c).is_none())
                    {
                        wanted.push((row, col));
                    }
                }
            }
            wanted.extend_from_slice(plan);

            if let Some(&(row, col)) = wanted.iter().find(|&&(r, c)| sim.plant_at(r, c).is_none()) {
                let _ = sim.plant(PlantKind::PeaShooter, row, col);
            }
            sim.step(DT);
            if sim.outcome != Outcome::Playing {
                break;
            }
        }
        (sim.outcome, sim)
    }

    #[test]
    fn sin_defensa_se_pierde() {
        for seed in 0..5 {
            let mut sim = Sim::new(Level::test_level(), seed);
            run(&mut sim, 300.0);
            assert_eq!(sim.outcome, Outcome::Lost, "semilla {seed}");
        }
    }

    #[test]
    fn el_nivel_de_prueba_se_puede_ganar_con_una_defensa_razonable() {
        // El bot recoge cada sol al instante y solo planta donde ya hay zombies:
        // una persona atenta lo hace igual de bien o un poco peor. Si se cambian los
        // soles, las oleadas o las plantas del nivel, este es el termometro del balance.
        let mut wins = 0;
        let mut report = Vec::new();
        for seed in 0..40 {
            let (outcome, sim) = bot(Level::test_level(), seed, &[]);
            report.push((
                seed,
                outcome,
                sim.time as u32,
                sim.killed,
                sim.plants.len(),
                sim.sun,
            ));
            if outcome == Outcome::Won {
                wins += 1;
                assert_eq!(sim.killed, Level::test_level().total_zombies());
            }
        }
        assert!(wins >= 38, "solo se gano {wins}/40; resultados: {report:?}");
    }
}
