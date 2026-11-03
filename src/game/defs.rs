//! Definiciones de plantas y zombies. Para añadir uno nuevo: una variante en el
//! enum, su entrada en `plant_def`/`zombie_def` y su aspecto en `view.rs`.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlantKind {
    Sunflower,
    PeaShooter,
    SnowPea,
    Wallnut,
    PotatoMine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ZombieKind {
    Basic,
}

/// Todas las plantas que puede mostrar el selector.
pub const ALL_PLANTS: [PlantKind; 5] = [
    PlantKind::Sunflower,
    PlantKind::PeaShooter,
    PlantKind::Wallnut,
    PlantKind::SnowPea,
    PlantKind::PotatoMine,
];

/// Nivel (0-based) que hay que completar para desbloquear la planta.
/// `None` significa disponible desde el principio.
pub fn plant_unlock_level(kind: PlantKind) -> Option<usize> {
    match kind {
        PlantKind::Sunflower | PlantKind::PeaShooter => None,
        PlantKind::Wallnut => Some(0),
        PlantKind::SnowPea => Some(1),
        PlantKind::PotatoMine => Some(2),
    }
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
    /// Segundos que el impacto deja al zombie a media velocidad. 0 = guisante normal.
    pub chill: f32,
    /// Imagen del proyectil, relativa a `assets/`.
    pub projectile: &'static str,
}

/// Planta que suelta soles.
#[derive(Clone, Copy, Debug)]
pub struct ProducerDef {
    /// Segundos hasta el primer sol.
    pub first: f32,
    /// Segundos entre soles, a partir del segundo.
    pub interval: f32,
    pub value: u32,
}

/// Papa mina: los zombies la pisan sin comerla y, al armarse, revienta al primero.
#[derive(Clone, Copy, Debug)]
pub struct MineDef {
    /// Segundos bajo tierra antes de armarse.
    pub arm_time: f32,
    pub damage: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct PlantDef {
    pub kind: PlantKind,
    pub name: &'static str,
    pub cost: u32,
    /// Segundos hasta poder volver a elegir la semilla.
    pub cooldown: f32,
    pub max_hp: f32,
    /// Carpeta del `.reanim` y de las texturas.
    pub entity: &'static str,
    pub shooter: Option<ShooterDef>,
    pub producer: Option<ProducerDef>,
    pub mine: Option<MineDef>,
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
    let (name, cost, cooldown, max_hp, entity, shooter, producer, mine) = match kind {
        PlantKind::Sunflower => (
            "Girasol",
            50,
            7.5,
            300.0,
            "SunFlower",
            None,
            Some(ProducerDef {
                first: 7.5,
                interval: 24.0,
                value: 25,
            }),
            None,
        ),
        PlantKind::PeaShooter => (
            "Lanzaguisantes",
            100,
            7.5,
            300.0,
            "PeaShooter",
            Some(pea(0.0, "images/ProjectilePea.png")),
            None,
            None,
        ),
        PlantKind::SnowPea => (
            "Lanzaguisantes de hielo",
            175,
            7.5,
            300.0,
            "SnowPea",
            Some(pea(10.0, "images/ProjectileSnowPea.png")),
            None,
            None,
        ),
        PlantKind::Wallnut => ("Nuez", 50, 30.0, 4000.0, "Wallnut", None, None, None),
        PlantKind::PotatoMine => (
            "Papa mina",
            25,
            30.0,
            300.0,
            "PotatoMine",
            None,
            None,
            Some(MineDef {
                arm_time: 15.0,
                damage: 1800.0,
            }),
        ),
    };
    PlantDef {
        kind,
        name,
        cost,
        cooldown,
        max_hp,
        entity,
        shooter,
        producer,
        mine,
    }
}

fn pea(chill: f32, projectile: &'static str) -> ShooterDef {
    ShooterDef {
        interval: 1.425,
        fire_phase: 0.5,
        damage: 20.0,
        pea_speed: 333.0,
        chill,
        projectile,
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
