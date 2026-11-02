//! Definiciones de plantas y zombies. Para añadir uno nuevo: una variante en el
//! enum, su entrada en `plant_def`/`zombie_def` y su aspecto en `view.rs`.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlantKind {
    PeaShooter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ZombieKind {
    Basic,
}

/// Plantas que disparan guisantes en línea recta.
#[derive(Clone, Copy, Debug)]
pub struct ShooterDef {
    /// Segundos entre disparos.
    pub interval: f32,
    /// Punto del ciclo de animación (0..1) en el que sale el guisante.
    pub fire_phase: f32,
    pub damage: f32,
    /// Píxeles por segundo.
    pub pea_speed: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct PlantDef {
    pub kind: PlantKind,
    pub name: &'static str,
    pub cost: u32,
    /// Segundos hasta poder volver a elegir la semilla.
    pub cooldown: f32,
    pub max_hp: f32,
    pub shooter: Option<ShooterDef>,
}

#[derive(Clone, Copy, Debug)]
pub struct ZombieDef {
    pub kind: ZombieKind,
    pub name: &'static str,
    pub max_hp: f32,
    /// Píxeles por segundo.
    pub speed: f32,
    /// Daño por segundo a la planta que se está comiendo.
    pub bite_dps: f32,
    /// Duración de la animación de muerte, en segundos.
    pub death_time: f32,
    /// Zona del cuerpo que reciben los guisantes, relativa al origen del zombie.
    pub hit_left: f32,
    pub hit_right: f32,
    /// Distancia del origen al frente (manos) del zombie.
    pub bite_reach: f32,
}

pub fn plant_def(kind: PlantKind) -> PlantDef {
    match kind {
        PlantKind::PeaShooter => PlantDef {
            kind,
            name: "Lanzaguisantes",
            cost: 100,
            cooldown: 7.5,
            max_hp: 300.0,
            shooter: Some(ShooterDef {
                interval: 1.425,
                fire_phase: 0.5,
                damage: 20.0,
                pea_speed: 333.0,
            }),
        },
    }
}

pub fn zombie_def(kind: ZombieKind) -> ZombieDef {
    match kind {
        ZombieKind::Basic => ZombieDef {
            kind,
            name: "Zombi",
            max_hp: 270.0,
            speed: 18.5,
            bite_dps: 100.0,
            // anim_death: 39 frames a 12 fps.
            death_time: 39.0 / 12.0,
            hit_left: 35.0,
            hit_right: 75.0,
            bite_reach: 20.0,
        },
    }
}
