//! Dibuja la simulación: césped, plantas, zombi básico, guisantes y soles.
//!
//! La lógica vive en `sim`. Aquí solo se lee el estado, se mandan órdenes y se
//! crean o se quitan los sprites. Las coordenadas de pantalla están en `board`.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::character::{pose_part, spawn_parts};
use crate::components::CharacterPart;
use crate::game::board::{
    cell_at, cell_left, cell_top, screen_to_world, CELL_H, CELL_W, SCREEN_H, SCREEN_W,
};
use crate::game::defs::{plant_def, zombie_def, PlantKind};
use crate::game::level::Level;
use crate::game::sim::{
    Id, Outcome, PlantError, SeedStatus, Sim, SimEvent, ZombieStatus, PEA_WIDTH, PEA_Y_OFFSET,
};
use crate::peashooter::{state_index, STATE_IDLE, STATE_SHOOTING};
use crate::reanim::ReanimData;
use crate::resources::CharacterLibraries;
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
    /// Sobre elegido en el banco. Las teclas 1-5 también lo cambian.
    selected: usize,
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
            (input, simulate, apply_deferred, animate)
                .chain()
                .run_if(in_state(AppState::Play)),
        );
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((Camera2d, PlayPiece));

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
                            top: Val::Px(8.0),
                            width: Val::Px(34.0),
                            height: Val::Px(34.0),
                            ..default()
                        },
                        ImageNode::new(asset_server.load(def.icon)),
                    ));
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
            control.selected = index;
        }
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
        control.selected = index;
        return;
    }

    if let Some(id) = sim.0.sun_at(cursor.x, cursor.y) {
        sim.0.collect_sun(id);
        return;
    }
    let Some((row, col)) = cell_at(cursor.x, cursor.y) else {
        return;
    };
    let Some(kind) = sim.0.seeds.get(control.selected).map(|seed| seed.kind) else {
        return;
    };
    if let Err(error) = sim.0.plant(kind, row, col) {
        let name = plant_def(kind).name;
        control.notice = match error {
            PlantError::NotEnoughSun => "No hay soles suficientes".into(),
            PlantError::OnCooldown => format!("{name} se está recargando"),
            PlantError::CellOccupied => "Esa celda está ocupada".into(),
            PlantError::OutOfBoard | PlantError::NoSuchSeed | PlantError::GameOver => String::new(),
        };
        control.notice_until = time.elapsed_secs() + 1.6;
    }
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
        pose_children(
            &mut parts,
            children,
            data,
            index,
            local,
            plant_def(plant.kind).entity,
            None,
            &asset_server,
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
        );
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
            );
        }
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
                spawn_parts(commands, asset_server, &libraries.sun, "Sun", entity, false);
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
) {
    let Some(state) = data.states.get(index) else {
        return;
    };
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
    let open =
        sim.plant_at(row, col).is_none() && sim.seed_status(control.selected) == SeedStatus::Ready;
    sprite.color = if open {
        Color::srgba(0.45, 1.0, 0.4, 0.28)
    } else {
        Color::srgba(1.0, 0.35, 0.25, 0.28)
    };
    *vis = Visibility::Inherited;
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
    let chosen = sim
        .seeds
        .get(control.selected)
        .map(|seed| plant_def(seed.kind).name)
        .unwrap_or("");
    let line = format!(
        "{wave_text}    zombies {}/{}{flag}\n{chosen}    1-5 o clic en el sobre    P: pausa    R: reiniciar    Esc: menú{notice}",
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
        let selected = slot.0 == control.selected;
        image.color = match sim.seed_status(slot.0) {
            SeedStatus::Ready => Color::WHITE,
            SeedStatus::Cooldown => Color::srgb(0.45, 0.45, 0.45),
            SeedStatus::NoSun => Color::srgb(0.7, 0.55, 0.5),
        };
        border.0 = if selected {
            Color::srgb(1.0, 0.86, 0.25)
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.0)
        };
    }
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
    let level = Level::test_level();
    let selected = level
        .seeds
        .iter()
        .position(|kind| *kind == PlantKind::PeaShooter)
        .unwrap_or(0);
    PlayControl {
        paused: false,
        restart: false,
        clock: 0.0,
        banner: false,
        selected,
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
