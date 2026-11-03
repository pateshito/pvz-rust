mod assets;
mod character;
mod components;
mod game;
mod level_select;
mod menu;
mod peashooter;
mod plant_anim;
mod plant_select;
mod reanim;
mod resources;
mod systems;
mod utils;
mod viewer;
mod zombie_style;

use assets::catalog::EntityCatalog;
use bevy::prelude::*;
use bevy::render::view::VisibilitySystems;
use bevy::transform::TransformSystem;
use reanim::{parse_reanim, ReanimData};
use resources::{CharacterLibraries, GameCatalog, LevelProgress, PlantLoadout};
use std::path::{Path, PathBuf};
use utils::assets_dir;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Menu,
    LevelSelect,
    PlantSelect,
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

/// Carga el `.reanim` principal y, si la planta viene partida, une cuerpo y cabeza.
fn load_plant(catalog: &EntityCatalog, entity: &str) -> ReanimData {
    let mut data = load_reanim_or_exit(entity, &reanim_path(catalog, entity, entity));
    if let Err(e) = plant_anim::apply(entity, entity, &mut data) {
        eprintln!("{e}");
        std::process::exit(1);
    }
    data
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

    let zombie_data = load_reanim_or_exit("Zombie", &reanim_path(&catalog, "Zombie", "Zombie"));
    let peashooter_data = load_plant(&catalog, "PeaShooter");
    let snowpea_data = load_plant(&catalog, "SnowPea");
    let sunflower_data = load_plant(&catalog, "SunFlower");
    let wallnut_data = load_plant(&catalog, "Wallnut");
    let potatomine_data = load_plant(&catalog, "PotatoMine");
    let sun_data = load_reanim_or_exit("Sun", &reanim_path(&catalog, "Sun", "Sun"));

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "PVZ Rust".into(),
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
        snowpea: snowpea_data,
        sunflower: sunflower_data,
        wallnut: wallnut_data,
        potatomine: potatomine_data,
        sun: sun_data,
    })
    .insert_resource(GameCatalog(catalog))
    .insert_resource(LevelProgress::default())
    .insert_resource(PlantLoadout::default())
    .insert_resource(viewer::ReanimCache::default())
    // Tras calcular los `GlobalTransform` y antes de decidir qué se ve.
    .add_systems(
        PostUpdate,
        character::apply_part_shear
            .after(TransformSystem::TransformPropagate)
            .before(VisibilitySystems::CheckVisibility),
    )
    .add_systems(OnEnter(AppState::Menu), menu::setup_menu)
    .add_systems(OnExit(AppState::Menu), menu::cleanup_menu)
    .add_systems(
        Update,
        menu::menu_interaction.run_if(in_state(AppState::Menu)),
    )
    .add_systems(
        OnEnter(AppState::LevelSelect),
        level_select::setup_level_select,
    )
    .add_systems(
        OnExit(AppState::LevelSelect),
        level_select::cleanup_level_select,
    )
    .add_systems(
        Update,
        (
            level_select::level_select_input,
            level_select::level_select_interaction,
        )
            .run_if(in_state(AppState::LevelSelect)),
    )
    .add_systems(OnEnter(AppState::PlantSelect), plant_select::setup)
    .add_systems(OnExit(AppState::PlantSelect), plant_select::cleanup)
    .add_systems(
        Update,
        (
            plant_select::plant_select_input,
            plant_select::plant_select_interaction,
            apply_deferred,
            plant_select::paint_cards,
            plant_select::pose_icons,
        )
            .chain()
            .run_if(in_state(AppState::PlantSelect)),
    )
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
    );
    game::view::register(&mut app);
    app.run();
}
