use crate::assets::catalog::EntityCatalog;
use crate::reanim::ReanimData;
use crate::Character;
use bevy::prelude::*;

/// Catálogo de entidades descubierto al arrancar. Lo usa el modo test.
#[derive(Resource)]
pub struct GameCatalog(pub EntityCatalog);

#[derive(Resource)]
pub struct CharacterLibraries {
    pub zombie: ReanimData,
    pub peashooter: ReanimData,
}

impl CharacterLibraries {
    pub fn data(&self, character: Character) -> &ReanimData {
        match character {
            Character::Zombie => &self.zombie,
            Character::PeaShooter => &self.peashooter,
        }
    }
}

#[derive(Resource)]
pub struct ViewerState {
    pub current: usize,
    pub time: f32,
    pub paused: bool,
    pub moving: bool,
    pub entity: String,
    pub reanim_name: String,
    pub reanim_index: usize,
    pub reanim_count: usize,
    /// Índice en `PlainStyle::ALL`. Solo aplica si `plain` es true.
    pub style: usize,
    /// El `.reanim` abierto es el zombi normal, el que mezcla accesorios.
    pub plain: bool,
    /// Este frame se recreó el personaje: no posar las piezas viejas.
    pub skip_pose: bool,
    /// Escala del personaje en el visor.
    pub zoom: f32,
    pub spawned: usize,
    pub skipped: usize,
    /// Ids de imagen que no se pudieron resolver al crear las piezas.
    pub missing: Vec<String>,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            current: 0,
            time: 0.0,
            paused: false,
            moving: false,
            entity: "Zombie".into(),
            reanim_name: "Zombie".into(),
            reanim_index: 0,
            reanim_count: 1,
            style: 0,
            plain: true,
            skip_pose: false,
            zoom: 1.0,
            spawned: 0,
            skipped: 0,
            missing: Vec::new(),
        }
    }
}
