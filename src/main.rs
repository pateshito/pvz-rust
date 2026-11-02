mod assets;
mod character;
mod components;
mod game;
mod menu;
mod peashooter;
mod reanim;
mod resources;
mod systems;
mod utils;
mod viewer;
mod zombie_style;

use assets::catalog::EntityCatalog;
use bevy::prelude::*;
use reanim::{parse_reanim, ReanimData};
use resources::{CharacterLibraries, GameCatalog};
use std::path::{Path, PathBuf};
use utils::assets_dir;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Menu,
    Play,
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

fn reanim_path(catalog: &EntityCatalog, entity: &str, name: &str) -> PathBuf {
    catalog
        .entity(entity)
        .and_then(|e| e.reanim(name))
        .map(|r| r.path.clone())
        .unwrap_or_else(|| {
            eprintln!("❌ No existe el reanim '{name}' de la entidad '{entity}'");
            std::process::exit(1);
        })
}

fn main() {
    let assets = assets_dir();

    let catalog = EntityCatalog::discover(&assets).unwrap_or_else(|e| {
        eprintln!("{e}");
        eprintln!("   Carpeta de assets esperada: {:?}", assets);
        std::process::exit(1);
    });
    println!(
        "📚 Catálogo: {} entidades, {} .reanim",
        catalog.len(),
        catalog.reanim_count()
    );

    let zombie_path = reanim_path(&catalog, "Zombie", "Zombie");
    let peashooter_path = reanim_path(&catalog, "PeaShooter", "PeaShooter");

    let zombie_data = load_reanim_or_exit("Zombie", &zombie_path);
    let mut peashooter_data = load_reanim_or_exit("PeaShooter", &peashooter_path);
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
        .insert_resource(GameCatalog(catalog))
        .insert_resource(viewer::ReanimCache::default())
        .add_systems(OnEnter(AppState::Menu), menu::setup_menu)
        .add_systems(OnExit(AppState::Menu), menu::cleanup_menu)
        .add_systems(
            Update,
            menu::menu_interaction.run_if(in_state(AppState::Menu)),
        )
        .add_systems(OnEnter(AppState::Play), menu::setup_play)
        .add_systems(OnExit(AppState::Play), menu::cleanup_play)
        .add_systems(Update, menu::play_input.run_if(in_state(AppState::Play)))
        .add_systems(OnEnter(AppState::TestMode), viewer::setup_test_mode)
        .add_systems(OnExit(AppState::TestMode), viewer::cleanup_test_mode)
        .add_systems(
            Update,
            (
                viewer::navigate,
                viewer::switch_animation,
                viewer::play_animation,
                systems::move_characters,
                viewer::paint_entity_buttons,
            )
                .chain()
                .run_if(in_state(AppState::TestMode)),
        )
        .run();
}
