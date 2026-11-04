//! Definiciones de plantas y zombies. Para añadir uno nuevo: una variante en el
//! enum, su entrada en `plant_def`/`zombie_def` y su aspecto en `view.rs`.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlantKind {
    PeaShooter,
    Sunflower,
    CherryBomb,
    Wallnut,
    PotatoMine,
    SnowPea,
    Chomper,
    Repeater,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ZombieKind {
    Basic,
    Conehead,
    Buckethead,
}

/// Todas las plantas que puede mostrar el selector, en el orden de la etapa 1.
pub const ALL_PLANTS: [PlantKind; 8] = [
    PlantKind::PeaShooter,
    PlantKind::Sunflower,
    PlantKind::CherryBomb,
    PlantKind::Wallnut,
    PlantKind::PotatoMine,
    PlantKind::SnowPea,
    PlantKind::Chomper,
    PlantKind::Repeater,
];

/// Sobres que caben en el banco de semillas.
pub const MAX_LOADOUT: usize = 6;

/// Planta que hay que completar para desbloquear cada una.
/// `None` significa disponible desde el principio. Hay una sola planta nueva
/// por nivel, en el orden de la campaña.
pub fn plant_unlock_level(kind: PlantKind) -> Option<usize> {
    match kind {
        PlantKind::PeaShooter | PlantKind::Sunflower => None,
        PlantKind::CherryBomb => Some(0),
        PlantKind::Wallnut => Some(1),
        PlantKind::PotatoMine => Some(2),
        PlantKind::SnowPea => Some(3),
        PlantKind::Chomper => Some(4),
        PlantKind::Repeater => Some(5),
    }
}

/// Planta que se desbloquea al completar el nivel indicado.
pub fn plant_unlocked_by_level(level: usize) -> Option<PlantKind> {
    ALL_PLANTS
        .into_iter()
        .find(|kind| plant_unlock_level(*kind) == Some(level))
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
    /// Guisantes que salen en cada disparo.
    pub shots: u8,
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

/// Petacereza: mecha corta y explosión en área.
#[derive(Clone, Copy, Debug)]
pub struct BombDef {
    /// Segundos desde que se planta hasta que explota.
    pub fuse: f32,
    pub damage: f32,
    /// Radio en celdas. 1 equivale a un área de 3x3.
    pub radius: usize,
}

/// Planta carnívora: devora a un zombie y se queda masticando.
#[derive(Clone, Copy, Debug)]
pub struct ChomperDef {
    /// Segundos que tarda en tragar antes de poder volver a morder.
    pub chew_time: f32,
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
    pub bomb: Option<BombDef>,
    pub chomper: Option<ChomperDef>,
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
    let basic_pea = Some(pea(0.0, "images/ProjectilePea.png"));
    match kind {
        PlantKind::Sunflower => PlantDef {
            kind,
            name: "Girasol",
            cost: 50,
            cooldown: 7.5,
            max_hp: 300.0,
            entity: "SunFlower",
            shooter: None,
            producer: Some(ProducerDef {
                first: 7.5,
                interval: 24.0,
                value: 25,
            }),
            mine: None,
            bomb: None,
            chomper: None,
        },
        PlantKind::PeaShooter => PlantDef {
            kind,
            name: "Lanzaguisantes",
            cost: 100,
            cooldown: 7.5,
            max_hp: 300.0,
            entity: "PeaShooter",
            shooter: basic_pea,
            producer: None,
            mine: None,
            bomb: None,
            chomper: None,
        },
        PlantKind::CherryBomb => PlantDef {
            kind,
            name: "Petacereza",
            cost: 150,
            cooldown: 50.0,
            max_hp: 300.0,
            entity: "CherryBomb",
            shooter: None,
            producer: None,
            mine: None,
            bomb: Some(BombDef {
                fuse: 1.2,
                damage: 1800.0,
                radius: 1,
            }),
            chomper: None,
        },
        PlantKind::Wallnut => PlantDef {
            kind,
            name: "Nuez",
            cost: 50,
            cooldown: 30.0,
            max_hp: 4000.0,
            entity: "Wallnut",
            shooter: None,
            producer: None,
            mine: None,
            bomb: None,
            chomper: None,
        },
        PlantKind::PotatoMine => PlantDef {
            kind,
            name: "Papa mina",
            cost: 25,
            cooldown: 30.0,
            max_hp: 300.0,
            entity: "PotatoMine",
            shooter: None,
            producer: None,
            mine: Some(MineDef {
                arm_time: 15.0,
                damage: 1800.0,
            }),
            bomb: None,
            chomper: None,
        },
        PlantKind::SnowPea => PlantDef {
            kind,
            name: "Lanzaguisantes de hielo",
            cost: 175,
            cooldown: 7.5,
            max_hp: 300.0,
            entity: "SnowPea",
            shooter: Some(pea(10.0, "images/ProjectileSnowPea.png")),
            producer: None,
            mine: None,
            bomb: None,
            chomper: None,
        },
        PlantKind::Chomper => PlantDef {
            kind,
            name: "Planta carnívora",
            cost: 150,
            cooldown: 7.5,
            max_hp: 300.0,
            entity: "Chomper",
            shooter: None,
            producer: None,
            mine: None,
            bomb: None,
            chomper: Some(ChomperDef { chew_time: 42.0 }),
        },
        PlantKind::Repeater => PlantDef {
            kind,
            name: "Repetidora",
            cost: 200,
            cooldown: 7.5,
            max_hp: 300.0,
            entity: "PeaShooter",
            shooter: Some(ShooterDef {
                shots: 2,
                ..pea(0.0, "images/ProjectilePea.png")
            }),
            producer: None,
            mine: None,
            bomb: None,
            chomper: None,
        },
    }
}

fn pea(chill: f32, projectile: &'static str) -> ShooterDef {
    ShooterDef {
        interval: 1.425,
        fire_phase: 0.5,
        damage: 20.0,
        pea_speed: 333.0,
        chill,
        shots: 1,
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
        ZombieKind::Conehead => ZombieDef {
            kind,
            name: "Zombi con cono",
            max_hp: 420.0,
            speed: 18.5,
            bite_dps: 100.0,
            death_time: 39.0 / 12.0,
            hit_left: 35.0,
            hit_right: 75.0,
            bite_reach: 20.0,
        },
        ZombieKind::Buckethead => ZombieDef {
            kind,
            name: "Zombi con cubeta",
            max_hp: 950.0,
            speed: 18.5,
            bite_dps: 100.0,
            death_time: 39.0 / 12.0,
            hit_left: 35.0,
            hit_right: 75.0,
            bite_reach: 20.0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn los_cascos_aguantan_mas_golpes() {
        let basic = zombie_def(ZombieKind::Basic).max_hp;
        let cone = zombie_def(ZombieKind::Conehead).max_hp;
        let bucket = zombie_def(ZombieKind::Buckethead).max_hp;
        assert!(basic < cone, "el cono protege");
        assert!(cone < bucket, "la cubeta protege más");
    }
}
