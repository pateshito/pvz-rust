mod components;
mod menu;
mod reanim;
mod resources;
mod systems;
mod utils;
mod viewer;

use bevy::prelude::*;
use reanim::parse_reanim;
use resources::{CharacterLibraries, SelectedCharacter};
use std::path::PathBuf;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Menu,
    TestMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Character {
    Zombie,
    PeaShooter,
}

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let zombie_path = manifest.join("assets/animations/Zombie/Zombie.reanim");
    let peashooter_path = manifest.join("assets/animations/PeaShooter/PeaShooter.reanim");

    println!("🔍 Leyendo Zombie: {:?}", zombie_path);
    let zombie_data = parse_reanim(&zombie_path);

    println!("🔍 Leyendo PeaShooter: {:?}", peashooter_path);
    let peashooter_data = parse_reanim(&peashooter_path);

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "PVZ Rust - Reanim Viewer".into(),
                        resolution: (800.0_f32, 600.0_f32).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .init_state::<AppState>()
        .insert_resource(CharacterLibraries {
            zombie: zombie_data,
            peashooter: peashooter_data,
        })
        .insert_resource(SelectedCharacter(Character::Zombie))
        .add_systems(OnEnter(AppState::Menu), menu::setup_menu)
        .add_systems(OnExit(AppState::Menu), menu::cleanup_menu)
        .add_systems(
            Update,
            menu::menu_interaction.run_if(in_state(AppState::Menu)),
        )
        .add_systems(OnEnter(AppState::TestMode), viewer::setup_test_mode)
        .add_systems(OnExit(AppState::TestMode), viewer::cleanup_test_mode)
        .add_systems(
            Update,
            (
                viewer::switch_animation,
                viewer::play_animation,
                systems::move_characters,
            )
                .chain()
                .run_if(in_state(AppState::TestMode)),
        )
        .run();
}
