use bevy::prelude::*;

#[derive(Component)]
pub struct CharacterRoot {
    pub speed: f32,
}

#[derive(Component)]
pub struct CharacterPart {
    pub track: usize,
    pub current_image: Option<String>,
}

#[derive(Component)]
pub struct MenuButton;

#[derive(Component, Debug)]
pub enum MenuAction {
    TestZombie,
    TestPeaShooter,
}

#[derive(Component)]
pub struct BackButton;

#[derive(Component)]
pub struct HudText;
