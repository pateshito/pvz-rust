use crate::assets::catalog::EntityCatalog;
use crate::game::defs::{plant_unlock_level, PlantKind, MAX_LOADOUT};
use crate::game::level::LEVEL_COUNT;
use crate::reanim::ReanimData;
use crate::Character;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

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
    pub mower: ReanimData,
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
/// primero; el nivel `cleared` es el siguiente desbloqueado. También guarda el
/// equipamiento, en el orden de las plantas elegidas.
#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct LevelProgress {
    pub cleared: usize,
    pub selected: usize,
    #[serde(default)]
    pub loadout: Vec<PlantKind>,
}

impl LevelProgress {
    const FILE: &'static str = "progress.json";

    /// Lee el progreso de `saves/progress.json`. Si no existe o está roto,
    /// empieza desde cero. Las plantas del equipamiento que aún no estén
    /// desbloqueadas se quitan y el archivo se reescribe.
    pub fn load() -> Self {
        let path = crate::utils::saves_dir().join(Self::FILE);
        let mut progress = match Self::load_from(&path) {
            Ok(progress) => {
                println!("💾 Progreso cargado de {:?}", path);
                progress
            }
            Err(e) => {
                println!("💾 Sin progreso guardado ({}). Se empieza desde cero.", e);
                Self::default()
            }
        };
        progress.cleared = progress.cleared.min(LEVEL_COUNT);
        progress.selected = progress.selected.min(LEVEL_COUNT.saturating_sub(1));
        if progress.sanitize_loadout() {
            println!("💾 Equipamiento ajustado a las plantas desbloqueadas.");
            progress.save();
        }
        progress
    }

    /// Quita del equipamiento las plantas bloqueadas o repetidas, respeta el
    /// orden guardado y deja el equipamiento por defecto si no queda ninguna.
    /// Devuelve `true` si hubo cambios.
    pub fn sanitize_loadout(&mut self) -> bool {
        let saved = self.loadout.clone();
        let mut clean: Vec<PlantKind> = Vec::new();
        for &kind in &saved {
            if clean.len() >= MAX_LOADOUT || clean.contains(&kind) {
                continue;
            }
            if self.plant_unlocked(kind) {
                clean.push(kind);
            }
        }
        if clean.is_empty() {
            clean = PlantLoadout::default().chosen;
        }
        self.loadout = clean;
        self.loadout != saved
    }

    fn load_from(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }

    /// Escribe el progreso en `saves/progress.json`, creando la carpeta si
    /// hace falta. Un fallo solo avisa: no debe cortar la partida.
    pub fn save(&self) {
        let dir = crate::utils::saves_dir();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("⚠️ No se pudo crear {:?}: {}", dir, e);
            return;
        }
        let path = dir.join(Self::FILE);
        match serde_json::to_string_pretty(self) {
            Ok(text) => {
                if let Err(e) = std::fs::write(&path, text) {
                    eprintln!("⚠️ No se pudo guardar el progreso en {:?}: {}", path, e);
                }
            }
            Err(e) => eprintln!("⚠️ No se pudo serializar el progreso: {}", e),
        }
    }

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
    /// Reconstruye el equipamiento con lo guardado en el progreso.
    pub fn from_progress(progress: &LevelProgress) -> Self {
        let chosen = if progress.loadout.is_empty() {
            Self::default().chosen
        } else {
            progress.loadout.clone()
        };
        Self { chosen }
    }

    /// Copia el equipamiento al progreso y lo guarda en disco.
    pub fn save_into(&self, progress: &mut LevelProgress) {
        progress.loadout = self.chosen.clone();
        progress.save();
    }

    pub fn toggle(&mut self, kind: PlantKind) -> Result<(), &'static str> {
        if let Some(index) = self.chosen.iter().position(|&chosen| chosen == kind) {
            if self.chosen.len() == 1 {
                return Err("Debes llevar al menos una planta.");
            }
            self.chosen.remove(index);
        } else {
            if self.chosen.len() >= MAX_LOADOUT {
                return Err("Solo puedes llevar 6 plantas.");
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
    fn el_equipamiento_se_guarda_con_el_progreso() {
        let progress = LevelProgress {
            cleared: 2,
            selected: 1,
            loadout: vec![PlantKind::Sunflower, PlantKind::CherryBomb],
        };
        let text = serde_json::to_string(&progress).unwrap();
        let back: LevelProgress = serde_json::from_str(&text).unwrap();
        assert_eq!(back.cleared, 2);
        assert_eq!(back.selected, 1);
        assert_eq!(back.loadout, progress.loadout);
        assert_eq!(PlantLoadout::from_progress(&back).chosen, progress.loadout);

        let viejo: LevelProgress = serde_json::from_str(r#"{"cleared":1,"selected":0}"#).unwrap();
        assert!(
            viejo.loadout.is_empty(),
            "un guardado viejo no trae equipamiento"
        );
    }

    #[test]
    fn el_equipamiento_quita_las_plantas_bloqueadas() {
        let mut progress = LevelProgress {
            cleared: 1,
            selected: 0,
            loadout: vec![
                PlantKind::CherryBomb, // desbloqueada al superar el nivel 0
                PlantKind::Wallnut,    // todavía bloqueada
                PlantKind::Sunflower,
                PlantKind::CherryBomb, // repetida
            ],
        };
        assert!(progress.sanitize_loadout());
        assert_eq!(
            progress.loadout,
            vec![PlantKind::CherryBomb, PlantKind::Sunflower],
            "conserva el orden y quita lo bloqueado"
        );
        assert!(!progress.sanitize_loadout());
    }

    #[test]
    fn un_equipamiento_vacio_vuelve_al_de_fabrica() {
        let mut progress = LevelProgress::default();
        assert!(progress.sanitize_loadout());
        assert_eq!(progress.loadout, PlantLoadout::default().chosen);
        assert!(!progress.sanitize_loadout());
    }

    #[test]
    fn el_equipamiento_no_pasa_de_seis_plantas() {
        let mut loadout = PlantLoadout::default();
        assert_eq!(loadout.chosen.len(), 2);
        assert!(loadout.toggle(PlantKind::CherryBomb).is_ok());
        assert!(loadout.toggle(PlantKind::Wallnut).is_ok());
        assert!(loadout.toggle(PlantKind::PotatoMine).is_ok());
        assert!(loadout.toggle(PlantKind::SnowPea).is_ok());
        assert_eq!(loadout.chosen.len(), MAX_LOADOUT);
        assert!(loadout.toggle(PlantKind::Chomper).is_err());
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
