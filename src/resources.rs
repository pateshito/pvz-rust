use crate::reanim::ReanimData;
use crate::Character;
use bevy::prelude::*;

#[derive(Resource)]
pub struct CharacterLibraries {
    pub zombie: ReanimData,
    pub peashooter: ReanimData,
}

#[derive(Resource)]
pub struct SelectedCharacter(pub Character);

#[derive(Resource)]
pub struct ViewerState {
    pub current: usize,
    pub time: f32,
    pub paused: bool,
    pub moving: bool,
    pub character: Character,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            current: 0,
            time: 0.0,
            paused: false,
            moving: false,
            character: Character::Zombie,
        }
    }
}
