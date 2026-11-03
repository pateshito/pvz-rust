//! Dibuja la simulación: césped, plantas, zombi básico, guisantes y soles.
//!
//! La lógica vive en `sim`. Aquí solo se lee el estado, se mandan órdenes y se
//! crean o se quitan los sprites. Las coordenadas de pantalla están en `board`.
//! Cada sobre del banco muestra el reposo completo de su planta, no un solo sprite.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use bevy::window::PrimaryWindow;

use crate::character::{pose_part, spawn_parts};
use crate::components::CharacterPart;
use crate::game::board::{
    cell_at, cell_left, cell_top, screen_to_world, CELL_H, CELL_W, SCREEN_H, SCREEN_W,
};
use crate::game::defs::{plant_def, zombie_def, PlantKind};
use crate::game::level::Level;
use crate::game::sim::{
    Id, Outcome, SeedStatus, Sim, SimEvent, ZombieStatus, PEA_WIDTH, PEA_Y_OFFSET,
};
use crate::peashooter::{state_index, STATE_IDLE, STATE_SHOOTING};
use crate::reanim::{ReanimData, BLINK_PERIOD};
use crate::resources::CharacterLibraries;
use crate::utils::{asset_exists, assets_dir, image_to_path};
use crate::zombie_style::PlainStyle;
use crate::AppState;

const BANK_X: f32 = 8.0;
const BANK_Y: f32 = 4.0;
const BANK_W: f32 = 446.0;
const BANK_H: f32 = 87.0;
const PACKET_X: f32 = 83.0;
const PACKET_Y: f32 = 8.0;
const PACKET_W: f32 = 50.0;
const PACKET_H: f32 = 70.0;
const PACKET_STEP: f32 = 52.0;
/// Margen del modelo dentro del sobre. Abajo queda el precio.
const ICON_INSET_X: f32 = 5.0;
const ICON_TOP: f32 = 4.0;
const ICON_BOTTOM: f32 = 16.0;
/// El banco es UI y se dibuja después del césped. El reposo del sobre y lo que
/// sigue al cursor van en esta capa, con una cámara que no limpia la pantalla.
const PREVIEW_LAYER: usize = 1;
/// Casilla de la pala, dentro del hueco derecho del banco.
const SHOVEL_W: f32 = 70.0;
const SHOVEL_H: f32 = 72.0;
const SHOVEL_X: f32 = BANK_X + BANK_W - 14.0 - SHOVEL_W;
const SHOVEL_Y: f32 = BANK_Y + (BANK_H - SHOVEL_H) * 0.5;
/// Origen del `.reanim` dentro de la celda. El lanzaguisantes en reposo, con el
/// ancla arriba a la izquierda, ocupa unos x 6..81 y 10..79: el centro visual
/// cae en (44, 45). La celda mide 80×100, así que el origen va en (-4, 6).
/// La boca, al disparar, queda entonces a la altura del guisante (y ≈ 36).
const PLANT_DX: f32 = -4.0;
const PLANT_DY: f32 = 6.0;
/// Los pies de `anim_walk` están cerca de y = 115. Así quedan justo sobre el
/// borde inferior del carril.
const ZOMBIE_DY: f32 = -21.0;

#[derive(Component)]
struct PlayPiece;

#[derive(Component)]
struct PlantActor(Id);

#[derive(Component)]
struct ZombieActor(Id);

#[derive(Component)]
struct PeaActor(Id);

#[derive(Component)]
struct SunActor(Id);

#[derive(Component)]
struct HoverCell;

#[derive(Component)]
struct StatusText;

#[derive(Component)]
struct SunText;

#[derive(Component)]
struct SeedCard;

#[derive(Component)]
struct SeedLabel;

#[derive(Component)]
struct SeedSlot(usize);

/// Reposo quieto dibujado encima de un sobre.
#[derive(Component)]
struct SeedPreview {
    slot: usize,
    kind: PlantKind,
}

/// Lo que el jugador lleva en el cursor. Se gasta al usarlo.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Hand {
    Empty,
    Seed(usize),
    Shovel,
}

#[derive(Component)]
struct ShovelButton;

#[derive(Component)]
struct ShovelIcon;

/// Planta a tamaño real pegada al cursor.
#[derive(Component)]
struct CursorPlant {
    kind: PlantKind,
    /// Centro visual del reposo, en píxeles del reanim (y hacia abajo).
    center: Vec2,
}

#[derive(Component)]
struct CursorShovel;

#[derive(Component)]
struct Banner;

#[derive(Resource)]
struct Match(Sim);

#[derive(Resource)]
struct PlayControl {
    paused: bool,
    restart: bool,
    /// Tiempo de animación. Se detiene con la pausa y al terminar la partida.
    clock: f32,
    banner: bool,
    /// Sobre o pala pegados al cursor. Vacío hasta que se elige uno.
    hand: Hand,
    notice: String,
    notice_until: f32,
}

#[derive(Resource, Default)]
struct Actors {
    plants: HashMap<Id, Entity>,
    zombies: HashMap<Id, Entity>,
    peas: HashMap<Id, Entity>,
    suns: HashMap<Id, Entity>,
}

impl Actors {
    fn despawn_all(&mut self, commands: &mut Commands) {
        for (_, entity) in self
            .plants
            .drain()
            .chain(self.zombies.drain())
            .chain(self.peas.drain())
            .chain(self.suns.drain())
        {
            commands.entity(entity).despawn_recursive();
        }
    }

    fn remove(&mut self, commands: &mut Commands, id: Id) {
        for map in [
            &mut self.plants,
            &mut self.zombies,
            &mut self.peas,
            &mut self.suns,
        ] {
            if let Some(entity) = map.remove(&id) {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

pub fn register(app: &mut App) {
    app.add_systems(OnEnter(AppState::Play), setup)
        .add_systems(OnExit(AppState::Play), cleanup)
        .add_systems(
            Update,
            (input, simulate, sync_hand, apply_deferred, animate)
                .chain()
                .run_if(in_state(AppState::Play)),
        );
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    libraries: Res<CharacterLibraries>,
) {
    // La UI se queda en esta cámara. La otra solo dibuja el modelo del sobre.
    commands.spawn((Camera2d, IsDefaultUiCamera, PlayPiece));
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::layer(PREVIEW_LAYER),
        PlayPiece,
    ));

    // `background1.jpg` mide 1400×600. La ventana de 800×600 se ve desde x = 220,
    // así que el centro de la foto queda en la x de pantalla 480.
    let (bx, by) = screen_to_world(480.0, 300.0);
    commands.spawn((
        Sprite {
            image: asset_server.load("images/background1.jpg"),
            ..default()
        },
        Transform::from_xyz(bx, by, -1.0),
        PlayPiece,
    ));

    commands.spawn((
        Sprite::from_color(
            Color::srgba(0.5, 1.0, 0.4, 0.0),
            Vec2::new(CELL_W - 6.0, CELL_H - 8.0),
        ),
        Transform::from_xyz(0.0, 0.0, -0.5),
        Visibility::Hidden,
        HoverCell,
        PlayPiece,
    ));

    let level = Level::test_level();
    // Banco de semillas: la imagen del juego, el contador de soles a la
    // izquierda y un sobre por planta.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(BANK_X),
                top: Val::Px(BANK_Y),
                width: Val::Px(BANK_W),
                height: Val::Px(BANK_H),
                ..default()
            },
            ImageNode::new(asset_server.load("images/SeedBank.png")),
            PlayPiece,
        ))
        .with_children(|bank| {
            bank.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(64.0),
                    width: Val::Px(78.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Text::new(level.start_sun.to_string()),
                TextFont::from_font_size(17.0),
                TextColor(Color::srgb(0.1, 0.07, 0.02)),
                TextLayout::new_with_justify(JustifyText::Center),
                SunText,
            ));
            for (index, kind) in level.seeds.iter().copied().enumerate() {
                let def = plant_def(kind);
                bank.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(PACKET_X + PACKET_STEP * index as f32),
                        top: Val::Px(PACKET_Y),
                        width: Val::Px(PACKET_W),
                        height: Val::Px(PACKET_H),
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(Val::Px(3.0)),
                        ..default()
                    },
                    ImageNode::new(asset_server.load("images/SeedPacket_Larger.png")),
                    BorderColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                    SeedCard,
                    SeedSlot(index),
                ))
                .with_children(|packet| {
                    packet.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            bottom: Val::Px(2.0),
                            ..default()
                        },
                        Text::new(def.cost.to_string()),
                        TextFont::from_font_size(12.0),
                        TextColor(Color::srgb(0.1, 0.07, 0.02)),
                        SeedLabel,
                        SeedSlot(index),
                    ));
                });
            }
        });

    // Panel de información, a la derecha del banco.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(BANK_W + 24.0),
                top: Val::Px(8.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.12, 0.08, 0.03, 0.78)),
            BorderRadius::all(Val::Px(8.0)),
            PlayPiece,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(""),
                TextFont::from_font_size(14.0),
                TextColor(Color::srgb(0.96, 0.92, 0.75)),
                StatusText,
            ));
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(SHOVEL_X),
                top: Val::Px(SHOVEL_Y),
                width: Val::Px(SHOVEL_W),
                height: Val::Px(SHOVEL_H),
                border: UiRect::all(Val::Px(3.0)),
                ..default()
            },
            ImageNode::new(asset_server.load("images/ShovelBank.png")),
            BorderColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            ShovelButton,
            PlayPiece,
        ))
        .with_children(|slot| {
            slot.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px((SHOVEL_W - 80.0) * 0.5),
                    top: Val::Px((SHOVEL_H - 80.0) * 0.5),
                    width: Val::Px(80.0),
                    height: Val::Px(80.0),
                    ..default()
                },
                ImageNode::new(asset_server.load("images/Shovel.png")),
                ShovelIcon,
            ));
        });

    commands.spawn((
        Sprite {
            image: asset_server.load("images/Shovel.png"),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 41.0),
        Visibility::Hidden,
        RenderLayers::layer(PREVIEW_LAYER),
        CursorShovel,
        PlayPiece,
    ));

    spawn_seed_previews(&mut commands, &asset_server, &libraries, &level);

    commands.insert_resource(Match(Sim::new(level, fresh_seed(0.0))));
    commands.insert_resource(fresh_control());
    commands.insert_resource(Actors::default());
}

fn cleanup(mut commands: Commands, query: Query<Entity, With<PlayPiece>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
    commands.remove_resource::<Match>();
    commands.remove_resource::<PlayControl>();
    commands.remove_resource::<Actors>();
}

fn input(
    mut next_state: ResMut<NextState<AppState>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut sim: ResMut<Match>,
    mut control: ResMut<PlayControl>,
    time: Res<Time>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::Menu);
        return;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        control.restart = true;
        return;
    }
    if keys.just_pressed(KeyCode::KeyP) && sim.0.outcome == Outcome::Playing {
        control.paused = !control.paused;
    }
    let now = time.elapsed_secs();
    for (index, key) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
    ]
    .into_iter()
    .enumerate()
    {
        if keys.just_pressed(key) && index < sim.0.seeds.len() {
            take_seed(&mut control, &sim.0, index, now);
        }
    }
    if mouse.just_pressed(MouseButton::Right) {
        control.hand = Hand::Empty;
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    if control.paused || sim.0.outcome != Outcome::Playing {
        return;
    }
    let Some(cursor) = cursor_on_board(windows.iter().next()) else {
        return;
    };
    if let Some(index) = packet_at(cursor, sim.0.seeds.len()) {
        take_seed(&mut control, &sim.0, index, now);
        return;
    }
    if shovel_at(cursor) {
        control.hand = if control.hand == Hand::Shovel {
            Hand::Empty
        } else {
            Hand::Shovel
        };
        return;
    }

    if let Some(id) = sim.0.sun_at(cursor.x, cursor.y) {
        sim.0.collect_sun(id);
        return;
    }
    match control.hand {
        Hand::Empty => {}
        Hand::Shovel => {
            let Some((row, col)) = cell_at(cursor.x, cursor.y) else {
                return;
            };
            if sim.0.dig(row, col).is_some() {
                control.hand = Hand::Empty;
            }
        }
        Hand::Seed(index) => {
            let Some((row, col)) = cell_at(cursor.x, cursor.y) else {
                return;
            };
            let Some(kind) = sim.0.seeds.get(index).map(|seed| seed.kind) else {
                control.hand = Hand::Empty;
                return;
            };
            if sim.0.plant(kind, row, col).is_ok() {
                control.hand = Hand::Empty;
            } else {
                let name = plant_def(kind).name;
                notify(
                    &mut control,
                    now,
                    match sim.0.seed_status(index) {
                        SeedStatus::NoSun => "No hay soles suficientes".into(),
                        SeedStatus::Cooldown => format!("{name} se está recargando"),
                        SeedStatus::Ready => "Esa celda está ocupada".into(),
                    },
                );
            }
        }
    }
}

/// Coge el sobre, o lo suelta si ya iba en el cursor. En recarga o sin soles no se coge.
fn take_seed(control: &mut PlayControl, sim: &Sim, index: usize, now: f32) {
    if control.hand == Hand::Seed(index) {
        control.hand = Hand::Empty;
        return;
    }
    let Some(seed) = sim.seeds.get(index) else {
        return;
    };
    let name = plant_def(seed.kind).name;
    match sim.seed_status(index) {
        SeedStatus::Ready => control.hand = Hand::Seed(index),
        SeedStatus::Cooldown => notify(control, now, format!("{name} se está recargando")),
        SeedStatus::NoSun => notify(control, now, "No hay soles suficientes".into()),
    }
}

fn notify(control: &mut PlayControl, now: f32, text: String) {
    control.notice = text;
    control.notice_until = now + 1.6;
}

fn simulate(
    time: Res<Time>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    libraries: Res<CharacterLibraries>,
    mut sim: ResMut<Match>,
    mut control: ResMut<PlayControl>,
    mut actors: ResMut<Actors>,
    banners: Query<Entity, With<Banner>>,
    mut hover: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<HoverCell>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut status: Query<&mut Text, With<StatusText>>,
    mut seed_label: Query<
        (&SeedSlot, &mut Text),
        (With<SeedLabel>, Without<StatusText>, Without<SunText>),
    >,
    mut sun_text: Query<&mut Text, (With<SunText>, Without<StatusText>, Without<SeedLabel>)>,
    mut seed_color: Query<(&SeedSlot, &mut ImageNode, &mut BorderColor), With<SeedCard>>,
) {
    if control.restart {
        actors.despawn_all(&mut commands);
        for entity in &banners {
            commands.entity(entity).despawn_recursive();
        }
        sim.0 = Sim::new(Level::test_level(), fresh_seed(sim.0.time));
        *control = fresh_control();
    }

    let playing = sim.0.outcome == Outcome::Playing && !control.paused;
    if playing {
        let dt = time.delta_secs();
        control.clock += dt;
        sim.0.step(dt);
    }

    for event in sim.0.take_events() {
        apply_event(
            &mut commands,
            &asset_server,
            &libraries,
            &sim.0,
            &mut actors,
            event,
        );
    }

    if sim.0.outcome != Outcome::Playing && !control.banner {
        spawn_banner(&mut commands, sim.0.outcome);
        control.banner = true;
    }

    place_hover(&sim.0, &control, windows.iter().next(), &mut hover);
    paint_hud(
        &sim.0,
        &control,
        time.elapsed_secs(),
        &mut status,
        &mut seed_label,
        &mut sun_text,
        &mut seed_color,
    );
}

fn animate(
    sim: Res<Match>,
    control: Res<PlayControl>,
    libraries: Res<CharacterLibraries>,
    asset_server: Res<AssetServer>,
    mut plants: Query<
        (&PlantActor, &Children, &mut Transform),
        (Without<CharacterPart>, Without<ZombieActor>),
    >,
    mut zombies: Query<
        (&ZombieActor, &Children, &mut Transform),
        (Without<CharacterPart>, Without<PlantActor>),
    >,
    mut parts: Query<(
        &mut Transform,
        &mut Sprite,
        &mut Visibility,
        &mut CharacterPart,
    )>,
    mut peas: Query<
        (&PeaActor, &mut Transform),
        (
            Without<CharacterPart>,
            Without<PlantActor>,
            Without<ZombieActor>,
        ),
    >,
    mut suns: Query<
        (&SunActor, &Children, &mut Transform),
        (
            Without<CharacterPart>,
            Without<PlantActor>,
            Without<ZombieActor>,
            Without<PeaActor>,
        ),
    >,
    previews: Query<(&SeedPreview, &Children)>,
    cursor_plants: Query<(&CursorPlant, &Children)>,
) {
    for (actor, children, mut root) in &mut plants {
        let Some(plant) = sim.0.plants.iter().find(|plant| plant.id == actor.0) else {
            continue;
        };
        let (x, y) = screen_to_world(
            cell_left(plant.col) + PLANT_DX,
            cell_top(plant.row) + PLANT_DY,
        );
        root.translation.x = x;
        root.translation.y = y;
        root.translation.z = plant.row as f32;
        let data = libraries.plant(plant.kind);
        let (index, local) = plant_frame(data, plant, control.clock);
        let phase = (plant.row as f32 * 1.7 + plant.col as f32 * 0.9) % BLINK_PERIOD;
        pose_children(
            &mut parts,
            children,
            data,
            index,
            local,
            plant_def(plant.kind).entity,
            None,
            &asset_server,
            control.clock,
            phase,
        );
    }

    let zombie_data = &libraries.zombie;
    for (actor, children, mut root) in &mut zombies {
        let Some(zombie) = sim.0.zombies.iter().find(|zombie| zombie.id == actor.0) else {
            continue;
        };
        let (x, y) = screen_to_world(zombie.x, cell_top(zombie.row) + ZOMBIE_DY);
        root.translation.x = x;
        root.translation.y = y;
        root.translation.z = zombie.row as f32 + 0.45;
        let (index, local) = zombie_frame(zombie_data, zombie, control.clock);
        pose_children(
            &mut parts,
            children,
            zombie_data,
            index,
            local,
            "Zombie",
            Some(PlainStyle::Normal),
            &asset_server,
            control.clock,
            0.0,
        );
        let tint = zombie_tint(zombie.chill);
        let mut iter = parts.iter_many_mut(children);
        while let Some((_, mut sprite, _, _)) = iter.fetch_next() {
            sprite.color = tint;
        }
    }

    for (actor, mut xf) in &mut peas {
        if let Some(pea) = sim.0.peas.iter().find(|pea| pea.id == actor.0) {
            let (x, y) = screen_to_world(pea.x + PEA_WIDTH * 0.5, cell_top(pea.row) + PEA_Y_OFFSET);
            xf.translation.x = x;
            xf.translation.y = y;
        }
    }
    let sun_data = &libraries.sun;
    for (actor, children, mut xf) in &mut suns {
        if let Some(sun) = sim.0.suns.iter().find(|sun| sun.id == actor.0) {
            let (x, y) = screen_to_world(sun.x, sun.y);
            xf.translation.x = x;
            xf.translation.y = y;
            let (index, local) = frame_in(
                sun_data,
                "full",
                (control.clock * sun_data.fps) as usize,
                true,
            );
            pose_children(
                &mut parts,
                children,
                sun_data,
                index,
                local,
                "Sun",
                None,
                &asset_server,
                control.clock,
                0.0,
            );
        }
    }

    for (preview, children) in &previews {
        let data = libraries.plant(preview.kind);
        let (index, local) = frame_in(data, preview_state(preview.kind), 0, false);
        pose_children(
            &mut parts,
            children,
            data,
            index,
            local,
            plant_def(preview.kind).entity,
            None,
            &asset_server,
            0.0,
            0.0,
        );
        let tint = seed_tint(sim.0.seed_status(preview.slot));
        let mut iter = parts.iter_many_mut(children);
        while let Some((_, mut sprite, _, _)) = iter.fetch_next() {
            sprite.color = tint;
        }
    }

    for (cursor, children) in &cursor_plants {
        let data = libraries.plant(cursor.kind);
        let (index, local) = frame_in(data, preview_state(cursor.kind), 0, false);
        pose_children(
            &mut parts,
            children,
            data,
            index,
            local,
            plant_def(cursor.kind).entity,
            None,
            &asset_server,
            0.0,
            0.0,
        );
    }
}

fn apply_event(
    commands: &mut Commands,
    asset_server: &AssetServer,
    libraries: &CharacterLibraries,
    sim: &Sim,
    actors: &mut Actors,
    event: SimEvent,
) {
    match event {
        SimEvent::PlantPlaced(id) => {
            if let Some(entity) = spawn_plant(commands, asset_server, libraries, sim, id) {
                actors.plants.insert(id, entity);
            }
        }
        SimEvent::ZombieSpawned(id) => {
            if let Some(entity) = spawn_zombie(commands, asset_server, libraries, sim, id) {
                actors.zombies.insert(id, entity);
            }
        }
        SimEvent::PeaFired(id) => {
            if let Some(pea) = sim.peas.iter().find(|pea| pea.id == id) {
                let (x, y) =
                    screen_to_world(pea.x + PEA_WIDTH * 0.5, cell_top(pea.row) + PEA_Y_OFFSET);
                let image = if pea.chill_for > 0.0 {
                    "images/ProjectileSnowPea.png"
                } else {
                    "images/ProjectilePea.png"
                };
                let entity = commands
                    .spawn((
                        Sprite {
                            image: asset_server.load(image),
                            custom_size: Some(Vec2::new(PEA_WIDTH, PEA_WIDTH)),
                            ..default()
                        },
                        Transform::from_xyz(x, y, pea.row as f32 + 0.7),
                        PeaActor(id),
                        PlayPiece,
                    ))
                    .id();
                actors.peas.insert(id, entity);
            }
        }
        SimEvent::SunSpawned(id) => {
            if let Some(sun) = sim.suns.iter().find(|sun| sun.id == id) {
                let (x, y) = screen_to_world(sun.x, sun.y);
                let entity = commands
                    .spawn((
                        Transform::from_xyz(x, y, 12.0),
                        Visibility::default(),
                        SunActor(id),
                        PlayPiece,
                    ))
                    .id();
                spawn_parts(
                    commands,
                    asset_server,
                    &libraries.sun,
                    "Sun",
                    entity,
                    false,
                    None,
                );
                actors.suns.insert(id, entity);
            }
        }
        SimEvent::PlantDied(id)
        | SimEvent::ZombieRemoved(id)
        | SimEvent::PeaRemoved { id, .. }
        | SimEvent::SunCollected(id)
        | SimEvent::SunExpired(id) => actors.remove(commands, id),
        SimEvent::ZombieDied(_) | SimEvent::Won | SimEvent::Lost => {}
    }
}

fn spawn_plant(
    commands: &mut Commands,
    asset_server: &AssetServer,
    libraries: &CharacterLibraries,
    sim: &Sim,
    id: Id,
) -> Option<Entity> {
    let plant = sim.plants.iter().find(|plant| plant.id == id)?;
    let (x, y) = screen_to_world(
        cell_left(plant.col) + PLANT_DX,
        cell_top(plant.row) + PLANT_DY,
    );
    let def = plant_def(plant.kind);
    let root = commands
        .spawn((
            Transform::from_xyz(x, y, plant.row as f32),
            Visibility::default(),
            PlantActor(id),
            PlayPiece,
        ))
        .id();
    spawn_parts(
        commands,
        asset_server,
        libraries.plant(plant.kind),
        def.entity,
        root,
        false,
        None,
    );
    Some(root)
}

fn spawn_zombie(
    commands: &mut Commands,
    asset_server: &AssetServer,
    libraries: &CharacterLibraries,
    sim: &Sim,
    id: Id,
) -> Option<Entity> {
    let zombie = sim.zombies.iter().find(|zombie| zombie.id == id)?;
    let (x, y) = screen_to_world(zombie.x, cell_top(zombie.row) + ZOMBIE_DY);
    let root = commands
        .spawn((
            Transform::from_xyz(x, y, zombie.row as f32 + 0.45),
            Visibility::default(),
            ZombieActor(id),
            PlayPiece,
        ))
        .id();
    spawn_parts(
        commands,
        asset_server,
        &libraries.zombie,
        "Zombie",
        root,
        false,
        None,
    );
    Some(root)
}

fn pose_children(
    parts: &mut Query<(
        &mut Transform,
        &mut Sprite,
        &mut Visibility,
        &mut CharacterPart,
    )>,
    children: &Children,
    data: &ReanimData,
    index: usize,
    local: usize,
    entity: &str,
    style: Option<PlainStyle>,
    asset_server: &AssetServer,
    time: f32,
    phase: f32,
) {
    let Some(state) = data.states.get(index) else {
        return;
    };
    let blink = state.blink_at(time, data.fps, phase);
    let mut iter = parts.iter_many_mut(children);
    while let Some((mut xf, mut sprite, mut vis, mut part)) = iter.fetch_next() {
        if part.track >= data.tracks.len() {
            *vis = Visibility::Hidden;
            continue;
        }
        pose_part(
            data,
            state,
            local,
            entity,
            style,
            asset_server,
            &mut xf,
            &mut sprite,
            &mut vis,
            &mut part,
            blink,
        );
    }
}

fn plant_frame(data: &ReanimData, plant: &crate::game::sim::Plant, clock: f32) -> (usize, usize) {
    match plant.kind {
        PlantKind::PeaShooter | PlantKind::SnowPea => {
            if let Some(phase) = plant.shoot_phase {
                let len = frame_len(data, STATE_SHOOTING) as f32;
                frame_in(
                    data,
                    STATE_SHOOTING,
                    (phase.clamp(0.0, 0.999) * len) as usize,
                    false,
                )
            } else {
                frame_in(data, STATE_IDLE, (clock * data.fps) as usize, true)
            }
        }
        PlantKind::Sunflower => frame_in(data, "full", (clock * data.fps) as usize, true),
        PlantKind::Wallnut => frame_in(data, "anim_idle", (clock * data.fps) as usize, true),
        PlantKind::PotatoMine => potato_frame(data, plant, clock),
    }
}

/// Enterrada en `anim_idle`, sale con `anim_rise` al final y, armada, parpadea.
fn potato_frame(data: &ReanimData, plant: &crate::game::sim::Plant, clock: f32) -> (usize, usize) {
    if plant.armed {
        return frame_in(data, "anim_armed", (clock * data.fps) as usize, true);
    }
    let rise_len = frame_len(data, "anim_rise");
    let rise_time = rise_len as f32 / data.fps.max(1.0);
    let Some(mine) = plant_def(plant.kind).mine else {
        return frame_in(data, "anim_idle", 0, false);
    };
    let until_arm = mine.arm_time - plant.timer;
    if until_arm > rise_time {
        frame_in(data, "anim_idle", 0, false)
    } else {
        let local = ((rise_time - until_arm) / rise_time * rise_len as f32) as usize;
        frame_in(data, "anim_rise", local, false)
    }
}

fn zombie_frame(
    data: &ReanimData,
    zombie: &crate::game::sim::Zombie,
    clock: f32,
) -> (usize, usize) {
    match zombie.status {
        ZombieStatus::Dying { elapsed } => {
            let len = frame_len(data, "anim_death");
            let def = zombie_def(zombie.kind);
            let local = if def.death_time <= 0.0 {
                len - 1
            } else {
                ((elapsed / def.death_time) * len as f32) as usize
            };
            frame_in(data, "anim_death", local, false)
        }
        ZombieStatus::Eating(_) => frame_in(
            data,
            "anim_eat",
            (clock * data.fps * zombie.pace()) as usize,
            true,
        ),
        ZombieStatus::Walking => frame_in(
            data,
            "anim_walk",
            (clock * data.fps * zombie.pace()) as usize,
            true,
        ),
    }
}

fn frame_in(data: &ReanimData, name: &str, local: usize, wrap: bool) -> (usize, usize) {
    let index = state_index(data, name).unwrap_or(0);
    let len = data
        .states
        .get(index)
        .map(|state| state.len.max(1))
        .unwrap_or(1);
    let local = if wrap {
        local % len
    } else {
        local.min(len - 1)
    };
    (index, local)
}

fn frame_len(data: &ReanimData, name: &str) -> usize {
    let index = state_index(data, name).unwrap_or(0);
    data.states
        .get(index)
        .map(|state| state.len.max(1))
        .unwrap_or(1)
}

fn sync_hand(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    libraries: Res<CharacterLibraries>,
    sim: Res<Match>,
    control: Res<PlayControl>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut plant_cursor: Query<
        (Entity, &CursorPlant, &mut Transform, &mut Visibility),
        (Without<CursorShovel>, Without<ShovelIcon>),
    >,
    mut shovel_cursor: Query<
        (&mut Transform, &mut Visibility),
        (
            With<CursorShovel>,
            Without<CursorPlant>,
            Without<ShovelIcon>,
        ),
    >,
    mut shovel_icon: Query<
        &mut Visibility,
        (
            With<ShovelIcon>,
            Without<CursorPlant>,
            Without<CursorShovel>,
        ),
    >,
    mut shovel_button: Query<&mut BorderColor, With<ShovelButton>>,
) {
    let cursor = cursor_on_board(windows.iter().next());
    let holding_shovel = control.hand == Hand::Shovel;
    for mut icon in &mut shovel_icon {
        *icon = if holding_shovel {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    for mut border in &mut shovel_button {
        border.0 = if holding_shovel {
            Color::srgb(1.0, 0.86, 0.25)
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.0)
        };
    }
    if let Ok((mut xf, mut vis)) = shovel_cursor.get_single_mut() {
        if holding_shovel {
            if let Some(cursor) = cursor {
                let (x, y) = screen_to_world(cursor.x, cursor.y);
                xf.translation.x = x;
                xf.translation.y = y;
                *vis = Visibility::Inherited;
            } else {
                *vis = Visibility::Hidden;
            }
        } else {
            *vis = Visibility::Hidden;
        }
    }

    let held = match control.hand {
        Hand::Seed(index) => sim.0.seeds.get(index).map(|seed| seed.kind),
        Hand::Empty | Hand::Shovel => None,
    };
    let mut kept = false;
    for (entity, plant, mut xf, mut vis) in &mut plant_cursor {
        if held == Some(plant.kind) && !kept {
            place_on_cursor(&mut xf, &mut vis, plant.center, cursor);
            kept = true;
        } else {
            commands.entity(entity).despawn_recursive();
        }
    }
    if let Some(kind) = held {
        if !kept {
            spawn_cursor_plant(&mut commands, &asset_server, &libraries, kind, cursor);
        }
    }
}

fn place_on_cursor(xf: &mut Transform, vis: &mut Visibility, center: Vec2, cursor: Option<Vec2>) {
    let Some(cursor) = cursor else {
        *vis = Visibility::Hidden;
        return;
    };
    let (x, y) = screen_to_world(cursor.x - center.x, cursor.y - center.y);
    xf.translation.x = x;
    xf.translation.y = y;
    *vis = Visibility::Inherited;
}

fn spawn_cursor_plant(
    commands: &mut Commands,
    asset_server: &AssetServer,
    libraries: &CharacterLibraries,
    kind: PlantKind,
    cursor: Option<Vec2>,
) {
    let def = plant_def(kind);
    let data = libraries.plant(kind);
    let center = visual_center(kind, data);
    let (x, y, vis) = match cursor {
        Some(cursor) => {
            let (x, y) = screen_to_world(cursor.x - center.x, cursor.y - center.y);
            (x, y, Visibility::Inherited)
        }
        None => (0.0, 0.0, Visibility::Hidden),
    };
    let root = commands
        .spawn((
            Transform::from_xyz(x, y, 40.0),
            vis,
            CursorPlant { kind, center },
            PlayPiece,
        ))
        .id();
    spawn_parts(
        commands,
        asset_server,
        data,
        def.entity,
        root,
        false,
        Some(PREVIEW_LAYER),
    );
}

fn visual_center(kind: PlantKind, data: &ReanimData) -> Vec2 {
    let bounds =
        idle_bounds(data, plant_def(kind).entity, preview_state(kind)).unwrap_or(ScreenBox {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 80.0,
            max_y: 80.0,
        });
    Vec2::new(
        (bounds.min_x + bounds.max_x) * 0.5,
        (bounds.min_y + bounds.max_y) * 0.5,
    )
}

fn place_hover(
    sim: &Sim,
    control: &PlayControl,
    window: Option<&Window>,
    hover: &mut Query<(&mut Transform, &mut Sprite, &mut Visibility), With<HoverCell>>,
) {
    let Ok((mut xf, mut sprite, mut vis)) = hover.get_single_mut() else {
        return;
    };
    let cell = (sim.outcome == Outcome::Playing && !control.paused)
        .then(|| cursor_on_board(window))
        .flatten()
        .and_then(|cursor| cell_at(cursor.x, cursor.y));
    let Some((row, col)) = cell else {
        *vis = Visibility::Hidden;
        return;
    };
    let (x, y) = screen_to_world(cell_left(col) + CELL_W * 0.5, cell_top(row) + CELL_H * 0.5);
    xf.translation.x = x;
    xf.translation.y = y;
    let occupied = sim.plant_at(row, col).is_some();
    let (show, color) = match control.hand {
        Hand::Seed(index) => {
            let open = !occupied && sim.seed_status(index) == SeedStatus::Ready;
            let color = if open {
                Color::srgba(0.45, 1.0, 0.4, 0.28)
            } else {
                Color::srgba(1.0, 0.35, 0.25, 0.28)
            };
            (true, color)
        }
        Hand::Shovel => (occupied, Color::srgba(0.95, 0.55, 0.2, 0.35)),
        Hand::Empty => (false, Color::WHITE),
    };
    if show {
        sprite.color = color;
        *vis = Visibility::Inherited;
    } else {
        *vis = Visibility::Hidden;
    }
}

/// Celeste mientras el guisante de hielo lo tiene ralentizado.
fn zombie_tint(chill: f32) -> Color {
    if chill > 0.0 {
        Color::srgb(0.55, 0.84, 1.0)
    } else {
        Color::WHITE
    }
}

fn seed_tint(status: SeedStatus) -> Color {
    match status {
        SeedStatus::Ready => Color::WHITE,
        SeedStatus::Cooldown => Color::srgb(0.45, 0.45, 0.45),
        SeedStatus::NoSun => Color::srgb(0.7, 0.55, 0.5),
    }
}

/// Reposo que se enseña en el sobre: el mismo con el que la planta espera en el césped.
/// La papa es el montículo de `anim_idle`, no la cara suelta.
fn preview_state(kind: PlantKind) -> &'static str {
    match kind {
        PlantKind::PeaShooter | PlantKind::SnowPea => STATE_IDLE,
        PlantKind::Sunflower => "full",
        PlantKind::Wallnut | PlantKind::PotatoMine => "anim_idle",
    }
}

fn spawn_seed_previews(
    commands: &mut Commands,
    asset_server: &AssetServer,
    libraries: &CharacterLibraries,
    level: &Level,
) {
    for (slot, kind) in level.seeds.iter().copied().enumerate() {
        let def = plant_def(kind);
        let data = libraries.plant(kind);
        let bounds = idle_bounds(data, def.entity, preview_state(kind)).unwrap_or(ScreenBox {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 70.0,
            max_y: 70.0,
        });
        let (scale, root) = fit_preview(slot, bounds);
        let (x, y) = screen_to_world(root.x, root.y);
        let entity = commands
            .spawn((
                Transform::from_xyz(x, y, 20.0).with_scale(Vec3::new(scale, scale, 1.0)),
                Visibility::default(),
                SeedPreview { slot, kind },
                PlayPiece,
            ))
            .id();
        spawn_parts(
            commands,
            asset_server,
            data,
            def.entity,
            entity,
            false,
            Some(PREVIEW_LAYER),
        );
    }
}

/// Caja del modelo, en píxeles de pantalla (origen arriba a la izquierda).
fn icon_box(slot: usize) -> ScreenBox {
    let min_x = BANK_X + PACKET_X + PACKET_STEP * slot as f32 + ICON_INSET_X;
    let min_y = BANK_Y + PACKET_Y + ICON_TOP;
    ScreenBox {
        min_x,
        min_y,
        max_x: min_x + PACKET_W - ICON_INSET_X * 2.0,
        max_y: min_y + PACKET_H - ICON_TOP - ICON_BOTTOM,
    }
}

/// Origen y escala para que `bounds` (espacio del `.reanim`, y hacia abajo) llene la caja.
fn fit_preview(slot: usize, bounds: ScreenBox) -> (f32, Vec2) {
    let target = icon_box(slot);
    let scale =
        (target.width() / bounds.width().max(1.0)).min(target.height() / bounds.height().max(1.0));
    let root = Vec2::new(
        (target.min_x + target.max_x) * 0.5 - (bounds.min_x + bounds.max_x) * 0.5 * scale,
        (target.min_y + target.max_y) * 0.5 - (bounds.min_y + bounds.max_y) * 0.5 * scale,
    );
    (scale, root)
}

/// Unión, a lo largo de todo el reposo, de las piezas visibles. Incluye rotación y sesgo.
/// El margen evita que el balanceo roce el borde del sobre.
fn idle_bounds(data: &ReanimData, entity: &str, state_name: &str) -> Option<ScreenBox> {
    let state = data.states.iter().find(|state| state.name == state_name)?;
    let mut sizes: HashMap<String, Option<(f32, f32)>> = HashMap::new();
    let mut bounds: Option<ScreenBox> = None;
    for local in 0..state.len.max(1) {
        for track in &data.tracks {
            if !track.has_image {
                continue;
            }
            let (frame, dx) = state.sample(track.group, local);
            let Some(resolved) = track.frames.get(frame).and_then(|frame| frame.as_ref()) else {
                continue;
            };
            if !resolved.active {
                continue;
            }
            let Some(image) = resolved.image.as_deref().or_else(|| {
                track
                    .frames
                    .iter()
                    .find_map(|frame| frame.as_ref().and_then(|frame| frame.image.as_deref()))
            }) else {
                continue;
            };
            let Some(rel) = image_to_path(image, entity) else {
                continue;
            };
            if !asset_exists(&rel) {
                continue;
            }
            let Some(size) = cached_png_size(&mut sizes, &rel) else {
                continue;
            };
            let (axis_x, axis_y) = resolved.axes();
            for (u, v) in [(0.0, 0.0), (size.0, 0.0), (0.0, size.1), (size.0, size.1)] {
                // `axes` está en el espacio de Bevy (y hacia arriba). La caja es de pantalla.
                let world_x = u * axis_x[0] - v * axis_y[0];
                let world_y = u * axis_x[1] - v * axis_y[1];
                let x = resolved.x + dx + world_x;
                let y = resolved.y - world_y;
                bounds
                    .get_or_insert_with(|| ScreenBox::point(x, y))
                    .include(x, y);
            }
        }
    }
    bounds.map(|bounds| bounds.pad(0.06))
}

fn cached_png_size(
    cache: &mut HashMap<String, Option<(f32, f32)>>,
    rel: &str,
) -> Option<(f32, f32)> {
    if let Some(size) = cache.get(rel) {
        return *size;
    }
    let size = png_size(rel);
    cache.insert(rel.to_string(), size);
    size
}

fn png_size(rel: &str) -> Option<(f32, f32)> {
    let bytes = std::fs::read(assets_dir().join(rel)).ok()?;
    if bytes.len() < 24 || bytes[0..8] != *b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?) as f32;
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?) as f32;
    (width > 0.0 && height > 0.0).then_some((width, height))
}

/// Rectángulo en píxeles de pantalla, origen arriba a la izquierda.
#[derive(Clone, Copy, Debug)]
struct ScreenBox {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl ScreenBox {
    fn point(x: f32, y: f32) -> Self {
        Self {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        }
    }

    fn include(&mut self, x: f32, y: f32) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    fn width(self) -> f32 {
        self.max_x - self.min_x
    }

    fn height(self) -> f32 {
        self.max_y - self.min_y
    }

    fn pad(self, fraction: f32) -> Self {
        let pad_x = self.width() * fraction;
        let pad_y = self.height() * fraction;
        Self {
            min_x: self.min_x - pad_x,
            min_y: self.min_y - pad_y,
            max_x: self.max_x + pad_x,
            max_y: self.max_y + pad_y,
        }
    }
}

fn paint_hud(
    sim: &Sim,
    control: &PlayControl,
    now: f32,
    status: &mut Query<&mut Text, With<StatusText>>,
    seed_label: &mut Query<
        (&SeedSlot, &mut Text),
        (With<SeedLabel>, Without<StatusText>, Without<SunText>),
    >,
    sun_text: &mut Query<&mut Text, (With<SunText>, Without<StatusText>, Without<SeedLabel>)>,
    seed_color: &mut Query<(&SeedSlot, &mut ImageNode, &mut BorderColor), With<SeedCard>>,
) {
    let waves = sim.level.waves.len();
    let started = sim
        .level
        .waves
        .iter()
        .filter(|wave| wave.time <= sim.time)
        .count();
    let wave_text = if let Some(next) = sim.level.waves.iter().find(|wave| wave.time > sim.time) {
        format!(
            "oleada {}/{} en {:.0}s",
            started + 1,
            waves,
            (next.time - sim.time).max(0.0)
        )
    } else {
        format!("oleada {waves}/{waves}")
    };
    let flag = if control.paused {
        "   [PAUSA]"
    } else {
        match sim.outcome {
            Outcome::Won => "   [VICTORIA]",
            Outcome::Lost => "   [DERROTA]",
            Outcome::Playing => "",
        }
    };
    let notice = if now < control.notice_until {
        format!("\n{}", control.notice)
    } else {
        String::new()
    };
    let chosen = match control.hand {
        Hand::Seed(index) => sim
            .seeds
            .get(index)
            .map(|seed| plant_def(seed.kind).name)
            .unwrap_or(""),
        Hand::Shovel => "Pala",
        Hand::Empty => "",
    };
    let line = format!(
        "{wave_text}    zombies {}/{}{flag}\n{chosen}    1-5 o clic, luego en el césped    clic derecho suelta    P: pausa    Esc: menú{notice}",
        sim.killed,
        sim.level.total_zombies(),
    );
    for mut text in status.iter_mut() {
        text.0 = line.clone();
    }

    for mut text in sun_text.iter_mut() {
        text.0 = sim.sun.to_string();
    }

    for (slot, mut text) in seed_label.iter_mut() {
        let cost = sim
            .seeds
            .get(slot.0)
            .map(|seed| plant_def(seed.kind).cost)
            .unwrap_or(0);
        text.0 = match sim.seed_status(slot.0) {
            SeedStatus::Cooldown => {
                let left = sim
                    .seeds
                    .get(slot.0)
                    .map(|seed| seed.cooldown_left)
                    .unwrap_or(0.0);
                format!("{left:.0}s")
            }
            SeedStatus::Ready | SeedStatus::NoSun => cost.to_string(),
        };
    }
    for (slot, mut image, mut border) in seed_color.iter_mut() {
        let selected = matches!(control.hand, Hand::Seed(index) if index == slot.0);
        image.color = seed_tint(sim.seed_status(slot.0));
        border.0 = if selected {
            Color::srgb(1.0, 0.86, 0.25)
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.0)
        };
    }
}

fn shovel_at(cursor: Vec2) -> bool {
    cursor.x >= SHOVEL_X
        && cursor.x <= SHOVEL_X + SHOVEL_W
        && cursor.y >= SHOVEL_Y
        && cursor.y <= SHOVEL_Y + SHOVEL_H
}

fn packet_at(cursor: Vec2, count: usize) -> Option<usize> {
    let x = cursor.x - BANK_X - PACKET_X;
    let y = cursor.y - BANK_Y - PACKET_Y;
    if x < 0.0 || y < 0.0 || y > PACKET_H {
        return None;
    }
    let index = (x / PACKET_STEP) as usize;
    (index < count && x - index as f32 * PACKET_STEP <= PACKET_W).then_some(index)
}

fn fresh_control() -> PlayControl {
    PlayControl {
        paused: false,
        restart: false,
        clock: 0.0,
        banner: false,
        hand: Hand::Empty,
        notice: String::new(),
        notice_until: 0.0,
    }
}

fn spawn_banner(commands: &mut Commands, outcome: Outcome) {
    let title = match outcome {
        Outcome::Won => "El jardín está a salvo",
        Outcome::Lost => "Los zombies llegaron a la casa",
        Outcome::Playing => return,
    };
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
            Banner,
            PlayPiece,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(format!("{title}\nR: reiniciar    Esc: menú")),
                TextFont::from_font_size(32.0),
                TextColor(Color::srgb(0.98, 0.95, 0.75)),
            ));
        });
}

fn cursor_on_board(window: Option<&Window>) -> Option<Vec2> {
    let window = window?;
    let pos = window.cursor_position()?;
    let size = window.size();
    Some(Vec2::new(
        pos.x - size.x * 0.5 + SCREEN_W * 0.5,
        pos.y - size.y * 0.5 + SCREEN_H * 0.5,
    ))
}

fn fresh_seed(extra: f32) -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs().wrapping_add(extra as u64))
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plant_anim;
    use crate::reanim::parse_reanim;
    use std::collections::HashSet;
    use std::path::Path;

    fn load(entity: &str) -> ReanimData {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("assets/animations/{entity}/{entity}.reanim"));
        let mut data = parse_reanim(&path).unwrap_or_else(|err| panic!("{entity}: {err}"));
        plant_anim::apply(entity, entity, &mut data)
            .unwrap_or_else(|err| panic!("{entity}: {err}"));
        data
    }

    fn imagenes_activas(data: &ReanimData, state_name: &str) -> usize {
        let state = data
            .states
            .iter()
            .find(|state| state.name == state_name)
            .unwrap_or_else(|| panic!("falta {state_name}"));
        let mut images = HashSet::new();
        for track in &data.tracks {
            if !track.has_image {
                continue;
            }
            let (frame, _) = state.sample(track.group, 0);
            let Some(resolved) = track.frames.get(frame).and_then(|frame| frame.as_ref()) else {
                continue;
            };
            if resolved.active {
                if let Some(image) = &resolved.image {
                    images.insert(image.clone());
                }
            }
        }
        images.len()
    }

    #[test]
    fn el_reposo_del_sobre_entra_en_el_icono() {
        for kind in [
            PlantKind::Sunflower,
            PlantKind::PeaShooter,
            PlantKind::SnowPea,
            PlantKind::Wallnut,
            PlantKind::PotatoMine,
        ] {
            let def = plant_def(kind);
            let data = load(def.entity);
            let state = preview_state(kind);
            let bounds = idle_bounds(&data, def.entity, state).unwrap_or_else(|| {
                panic!("{} sin caja de {state}", def.entity);
            });
            assert!(
                bounds.width().is_finite()
                    && bounds.height().is_finite()
                    && (20.0..150.0).contains(&bounds.width())
                    && (20.0..150.0).contains(&bounds.height()),
                "{} reposo {:.1}×{:.1}",
                def.entity,
                bounds.width(),
                bounds.height()
            );
            let (scale, root) = fit_preview(2, bounds);
            assert!(
                scale.is_finite() && scale > 0.0,
                "{} escala {scale}",
                def.entity
            );
            let target = icon_box(2);
            let min_x = root.x + bounds.min_x * scale;
            let max_x = root.x + bounds.max_x * scale;
            let min_y = root.y + bounds.min_y * scale;
            let max_y = root.y + bounds.max_y * scale;
            let eps = 0.08;
            assert!(
                min_x >= target.min_x - eps
                    && max_x <= target.max_x + eps
                    && min_y >= target.min_y - eps
                    && max_y <= target.max_y + eps,
                "{} sale del sobre",
                def.entity
            );
            let fill = ((max_x - min_x) / target.width()).max((max_y - min_y) / target.height());
            assert!(fill > 0.97, "{} llena {fill}", def.entity);
        }
    }

    #[test]
    fn el_hielo_tiñe_al_zombie_de_celeste() {
        assert_eq!(zombie_tint(0.0), Color::WHITE);
        let tint = zombie_tint(10.0).to_srgba();
        assert!(tint.blue > tint.green && tint.green > tint.red);
        assert!(tint.alpha > 0.9);
    }

    #[test]
    fn la_pala_tiene_su_casilla() {
        assert!(shovel_at(Vec2::new(SHOVEL_X + 8.0, SHOVEL_Y + 8.0)));
        assert!(!shovel_at(Vec2::new(SHOVEL_X - 2.0, SHOVEL_Y + 8.0)));
        assert!(!shovel_at(Vec2::new(
            BANK_X + PACKET_X + 10.0,
            BANK_Y + PACKET_Y + 10.0
        )));
    }

    #[test]
    fn el_sobre_junta_las_piezas_del_reposo() {
        let casos = [
            ("SunFlower", "full", 6),
            ("PeaShooter", STATE_IDLE, 4),
            ("SnowPea", STATE_IDLE, 4),
            ("PotatoMine", "anim_idle", 3),
            ("Wallnut", "anim_idle", 1),
        ];
        for (entity, state, minimo) in casos {
            let data = load(entity);
            let n = imagenes_activas(&data, state);
            assert!(
                n >= minimo,
                "{entity} en {state} tiene {n} imágenes (mínimo {minimo})"
            );
        }
    }
}
