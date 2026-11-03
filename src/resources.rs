use crate::assets::catalog::EntityCatalog;
use crate::game::defs::{plant_unlock_level, PlantKind, MAX_LOADOUT};
use crate::game::level::LEVEL_COUNT;
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
    pub repeater: ReanimData,
    pub snowpea: ReanimData,
    pub sunflower: ReanimData,
    pub wallnut: ReanimData,
    pub potatomine: ReanimData,
    pub cherrybomb: ReanimData,
    pub chomper: ReanimData,
    pub sun: ReanimData,
}

impl CharacterLibraries {
    pub fn data(&self, character: Character) -> &ReanimData {
        match character {
            Character::Zombie => &self.zombie,
            Character::PeaShooter => &self.peashooter,
        }
    }

    pub fn plant(&self, kind: PlantKind) -> &ReanimData {
        match kind {
            PlantKind::PeaShooter => &self.peashooter,
            PlantKind::Repeater => &self.repeater,
            PlantKind::SnowPea => &self.snowpea,
            PlantKind::Sunflower => &self.sunflower,
            PlantKind::Wallnut => &self.wallnut,
            PlantKind::PotatoMine => &self.potatomine,
            PlantKind::CherryBomb => &self.cherrybomb,
            PlantKind::Chomper => &self.chomper,
        }
    }
}

/// Progreso de la campaña. `cleared` cuenta los niveles superados desde el
/// primero; el nivel `cleared` es el siguiente desbloqueado.
#[derive(Resource, Debug, Clone, Default)]
pub struct LevelProgress {
    pub cleared: usize,
    pub selected: usize,
}

impl LevelProgress {
    pub fn is_unlocked(&self, index: usize) -> bool {
        index < LEVEL_COUNT && index <= self.cleared
    }

    pub fn is_cleared(&self, index: usize) -> bool {
        index < self.cleared
    }

    /// Marca el nivel como superado y desbloquea el siguiente.
    pub fn clear(&mut self, index: usize) {
        if index == self.cleared {
            self.cleared = (self.cleared + 1).min(LEVEL_COUNT);
        }
    }

    pub fn plant_unlocked(&self, kind: PlantKind) -> bool {
        match plant_unlock_level(kind) {
            None => true,
            Some(level) => self.is_cleared(level),
        }
    }
}

/// Plantas elegidas para el próximo nivel. Se guarda en el orden de `ALL_PLANTS`.
#[derive(Resource, Debug, Clone)]
pub struct PlantLoadout {
    pub chosen: Vec<PlantKind>,
}

impl Default for PlantLoadout {
    fn default() -> Self {
        Self {
            chosen: vec![PlantKind::Sunflower, PlantKind::PeaShooter],
        }
    }
}

impl PlantLoadout {
    pub fn toggle(&mut self, kind: PlantKind) -> Result<(), &'static str> {
        if let Some(index) = self.chosen.iter().position(|&chosen| chosen == kind) {
            if self.chosen.len() == 1 {
                return Err("Debes llevar al menos una planta.");
            }
            self.chosen.remove(index);
        } else {
            if self.chosen.len() >= MAX_LOADOUT {
                return Err("Solo puedes llevar 5 plantas.");
            }
            self.chosen.push(kind);
            self.chosen.sort_by_key(|plant| {
                crate::game::defs::ALL_PLANTS
                    .iter()
                    .position(|candidate| candidate == plant)
                    .unwrap_or(usize::MAX)
            });
        }
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::defs::plant_unlocked_by_level;

    #[test]
    fn el_progreso_desbloquea_en_orden() {
        let mut progress = LevelProgress::default();
        assert!(progress.is_unlocked(0));
        assert!(!progress.is_unlocked(1));

        progress.clear(1); // nivel no desbloqueado: no salta
        assert!(!progress.is_unlocked(1));

        progress.clear(0);
        assert!(progress.is_cleared(0));
        assert!(progress.is_unlocked(1));
        assert!(!progress.is_unlocked(2));
    }

    #[test]
    fn el_progreso_no_pasa_del_ultimo_nivel() {
        let mut progress = LevelProgress::default();
        for index in 0..LEVEL_COUNT {
            progress.clear(index);
        }
        assert_eq!(progress.cleared, LEVEL_COUNT);
        progress.clear(LEVEL_COUNT - 1);
        assert_eq!(progress.cleared, LEVEL_COUNT);
    }

    #[test]
    fn las_plantas_se_desbloquean_por_nivel() {
        let mut progress = LevelProgress::default();
        assert!(progress.plant_unlocked(PlantKind::Sunflower));
        assert!(progress.plant_unlocked(PlantKind::PeaShooter));
        assert!(!progress.plant_unlocked(PlantKind::CherryBomb));

        progress.clear(0);
        assert!(progress.plant_unlocked(PlantKind::CherryBomb));
        assert!(!progress.plant_unlocked(PlantKind::Wallnut));

        progress.clear(1);
        assert!(progress.plant_unlocked(PlantKind::Wallnut));
        assert!(!progress.plant_unlocked(PlantKind::PotatoMine));

        progress.clear(2);
        assert!(progress.plant_unlocked(PlantKind::PotatoMine));
        assert!(!progress.plant_unlocked(PlantKind::SnowPea));

        progress.clear(3);
        assert!(progress.plant_unlocked(PlantKind::SnowPea));
        assert!(!progress.plant_unlocked(PlantKind::Chomper));

        progress.clear(4);
        assert!(progress.plant_unlocked(PlantKind::Chomper));
        assert!(!progress.plant_unlocked(PlantKind::Repeater));

        progress.clear(5);
        assert!(progress.plant_unlocked(PlantKind::Repeater));
    }

    #[test]
    fn cada_nivel_desbloquea_una_sola_planta() {
        let mut seen = Vec::new();
        for level in 0..LEVEL_COUNT {
            let kind = plant_unlocked_by_level(level).expect("un nivel sin planta nueva");
            assert!(!seen.contains(&kind), "planta repetida en el nivel {level}");
            seen.push(kind);
        }
        assert_eq!(seen.len(), LEVEL_COUNT);
    }

    #[test]
    fn el_equipamiento_no_pasa_de_cinco_plantas() {
        let mut loadout = PlantLoadout::default();
        assert_eq!(loadout.chosen.len(), 2);
        assert!(loadout.toggle(PlantKind::CherryBomb).is_ok());
        assert!(loadout.toggle(PlantKind::Wallnut).is_ok());
        assert!(loadout.toggle(PlantKind::PotatoMine).is_ok());
        assert_eq!(loadout.chosen.len(), MAX_LOADOUT);
        assert!(loadout.toggle(PlantKind::SnowPea).is_err());
        assert_eq!(loadout.chosen.len(), MAX_LOADOUT);
    }

    #[test]
    fn el_equipamiento_no_se_queda_vacio() {
        let mut loadout = PlantLoadout::default();
        assert_eq!(loadout.chosen.len(), 2);
        assert!(loadout.toggle(PlantKind::Sunflower).is_ok());
        assert_eq!(loadout.chosen, vec![PlantKind::PeaShooter]);
        assert!(loadout.toggle(PlantKind::PeaShooter).is_err());
        assert_eq!(loadout.chosen, vec![PlantKind::PeaShooter]);
    }
}
