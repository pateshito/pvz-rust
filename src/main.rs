mod components;
mod menu;
mod peashooter;
mod reanim;
mod resources;
mod systems;
mod utils;
mod viewer;

use bevy::prelude::*;
use reanim::{parse_reanim, ReanimData};
use resources::{CharacterLibraries, SelectedCharacter};
use std::path::Path;
use utils::assets_dir;

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

/// Lee un `.reanim`; si falla, muestra el error y termina sin `panic!`.
fn load_reanim_or_exit(label: &str, path: &Path) -> ReanimData {
    println!("🔍 Leyendo {}: {:?}", label, path);
    match parse_reanim(path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("{e}");
            eprintln!(
                "   Carpeta de assets esperada: {:?} (se puede cambiar con BEVY_ASSET_ROOT)",
                assets_dir()
            );
            std::process::exit(1);
        }
    }
}

fn main() {
    let assets = assets_dir();

    let zombie_data =
        load_reanim_or_exit("Zombie", &assets.join("animations/Zombie/Zombie.reanim"));
    let mut peashooter_data = load_reanim_or_exit(
        "PeaShooter",
        &assets.join("animations/PeaShooter/PeaShooter.reanim"),
    );
    // El PeaShooter solo tiene 2 estados (idle / shooting) con la planta completa.
    if let Err(e) = peashooter::build_states(&mut peashooter_data) {
        eprintln!("{e}");
        std::process::exit(1);
    }

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
