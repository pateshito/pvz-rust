use crate::assets::catalog::EntityCatalog;
use crate::character::{pose_part, spawn_parts, SpawnReport};
use crate::components::{
    BackButton, CharacterPart, CharacterRoot, EntityList, EntityPick, HudText, PreviewRoot,
};
use crate::reanim::{parse_reanim, ReanimData};
use crate::resources::{GameCatalog, ViewerState};
use crate::utils::{asset_exists, hud_text, image_to_path};
use crate::zombie_style::PlainStyle;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Component)]
pub(crate) struct TestModeEntity;

/// Asta y bandera, hijas de la mano del zombi abanderado.
#[derive(Component)]
pub struct FlagPart {
    track: usize,
}

/// `.reanim` en reproducción. `flag` es el asta, solo para el zombi normal.
#[derive(Resource)]
pub struct ActiveReanim {
    pub data: ReanimData,
    pub flag: Option<ReanimData>,
}

/// Orden de la lista, para moverse con ↑/↓.
#[derive(Resource)]
pub struct EntityNames(Vec<String>);

/// Evita parsear otra vez un `.reanim` ya abierto.
#[derive(Resource, Default)]
pub struct ReanimCache {
    map: HashMap<(String, String), ReanimData>,
}

pub fn setup_test_mode(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    catalog: Res<GameCatalog>,
    mut cache: ResMut<ReanimCache>,
) {
    commands.spawn((Camera2d, TestModeEntity));

    let names: Vec<String> = catalog.0.entities().map(|e| e.name.clone()).collect();
    let initial = if names.iter().any(|n| n == "Zombie") {
        "Zombie".to_string()
    } else {
        names.first().cloned().unwrap_or_default()
    };

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Px(220.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.1, 0.08)),
            TestModeEntity,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Entidades"),
                TextFont::from_font_size(18.0),
                TextColor(Color::srgb(0.9, 0.9, 0.4)),
            ));
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    EntityList,
                    ScrollPosition::default(),
                ))
                .with_children(|list| {
                    for name in &names {
                        spawn_entity_button(list, name);
                    }
                });
        });

    commands
        .spawn((
            Button,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                top: Val::Px(10.0),
                width: Val::Px(120.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderColor(Color::srgb(0.6, 0.3, 0.3)),
            BackgroundColor(Color::srgb(0.4, 0.2, 0.2)),
            BackButton,
            TestModeEntity,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("← Menú"),
                TextFont::from_font_size(18.0),
                TextColor(Color::WHITE),
            ));
        });

    let mut state = ViewerState {
        entity: initial,
        ..default()
    };
    let loaded = load_selection(&catalog.0, &mut cache, &mut state, true);
    if let Some(loaded) = loaded.as_ref() {
        let report = spawn_preview(&mut commands, &asset_server, &state, loaded);
        remember_spawn(&mut state, report);
    }
    let hud = match &loaded {
        Some(loaded) => hud_text(&state, &loaded.data),
        None => format!("{}\nsin animación", state.entity),
    };
    commands.spawn((
        Text::new(hud),
        TextFont::from_font_size(16.0),
        TextColor(Color::srgb(0.9, 0.9, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(232.0),
            top: Val::Px(10.0),
            ..default()
        },
        HudText,
        TestModeEntity,
    ));
    if let Some(loaded) = loaded {
        commands.insert_resource(loaded);
    }

    commands.insert_resource(EntityNames(names));
    commands.insert_resource(state);
}

fn spawn_entity_button(parent: &mut ChildBuilder, name: &str) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(28.0),
                flex_shrink: 0.0,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.14, 0.18, 0.14)),
            EntityPick(name.to_string()),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(name.to_string()),
                TextFont::from_font_size(14.0),
                TextColor(Color::WHITE),
            ));
        });
}

pub fn cleanup_test_mode(mut commands: Commands, query: Query<Entity, With<TestModeEntity>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    commands.remove_resource::<ActiveReanim>();
    commands.remove_resource::<ViewerState>();
    commands.remove_resource::<EntityNames>();
    println!("🧹 Test mode limpiado");
}

/// Cambia de entidad (lista o ↑/↓), de archivo (RePág/AvPág) o de estilo (`[ ]`).
pub fn navigate(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    catalog: Res<GameCatalog>,
    mut cache: ResMut<ReanimCache>,
    names: Res<EntityNames>,
    input: Res<ButtonInput<KeyCode>>,
    mut wheels: EventReader<MouseWheel>,
    mut state: ResMut<ViewerState>,
    mut scroll: Query<&mut ScrollPosition, With<EntityList>>,
    mut roots: Query<(Entity, &mut Transform), With<PreviewRoot>>,
    mut picks: Query<(&Interaction, &EntityPick), (Changed<Interaction>, With<Button>)>,
    windows: Query<&Window>,
    active: Option<Res<ActiveReanim>>,
    mut text: Query<&mut Text, With<HudText>>,
) {
    let mut entity_changed = false;
    for (interaction, pick) in &mut picks {
        if *interaction == Interaction::Pressed && pick.0 != state.entity {
            state.entity = pick.0.clone();
            entity_changed = true;
        }
    }

    if !names.0.is_empty()
        && (input.just_pressed(KeyCode::ArrowUp) || input.just_pressed(KeyCode::ArrowDown))
    {
        let len = names.0.len();
        let current = names.0.iter().position(|n| n == &state.entity).unwrap_or(0);
        let next = if input.just_pressed(KeyCode::ArrowDown) {
            (current + 1) % len
        } else {
            (current + len - 1) % len
        };
        if names.0[next] != state.entity {
            state.entity = names.0[next].clone();
            entity_changed = true;
        }
        if let Ok(mut scroll) = scroll.get_single_mut() {
            // Cada fila son 28 px de botón + 4 de separación.
            let row = next as f32 * 32.0;
            let view = 540.0;
            if scroll.offset_y > row {
                scroll.offset_y = row;
            } else if scroll.offset_y + view < row + 28.0 {
                scroll.offset_y = (row + 28.0 - view).max(0.0);
            }
        }
    }

    let mut zoom_changed = false;
    for event in wheels.read() {
        let over_list = windows
            .iter()
            .find_map(|window| window.cursor_position())
            .map(|pos| pos.x <= 220.0)
            .unwrap_or(false);
        if over_list {
            let dy = match event.unit {
                MouseScrollUnit::Line => event.y * 32.0,
                MouseScrollUnit::Pixel => event.y,
            };
            if let Ok(mut scroll) = scroll.get_single_mut() {
                scroll.offset_y = (scroll.offset_y - dy).max(0.0);
            }
        } else if event.y != 0.0 {
            let steps = match event.unit {
                MouseScrollUnit::Line => event.y,
                MouseScrollUnit::Pixel => event.y / 32.0,
            };
            state.zoom = (state.zoom * 1.15_f32.powf(steps)).clamp(0.2, 6.0);
            zoom_changed = true;
        }
    }

    let mut reanim_changed = false;
    if state.reanim_count > 1
        && (input.just_pressed(KeyCode::PageUp) || input.just_pressed(KeyCode::PageDown))
    {
        let len = state.reanim_count;
        if input.just_pressed(KeyCode::PageDown) {
            state.reanim_index = (state.reanim_index + 1) % len;
        } else {
            state.reanim_index = (state.reanim_index + len - 1) % len;
        }
        reanim_changed = true;
    }

    if entity_changed || reanim_changed {
        if entity_changed {
            state.style = 0;
        }
        state.time = 0.0;
        // Las piezas viejas siguen en el mundo hasta que se aplican los comandos.
        state.skip_pose = true;
        let stale: Vec<Entity> = roots.iter().map(|(entity, _)| entity).collect();
        for entity in stale {
            commands.entity(entity).despawn_recursive();
        }
        if let Some(loaded) = load_selection(&catalog.0, &mut cache, &mut state, entity_changed) {
            let report = spawn_preview(&mut commands, &asset_server, &state, &loaded);
            remember_spawn(&mut state, report);
            set_hud(&mut text, &state, &loaded.data);
            commands.insert_resource(loaded);
        } else {
            state.spawned = 0;
            state.skipped = 0;
            state.missing.clear();
            commands.remove_resource::<ActiveReanim>();
        }
        return;
    }

    let zoom_in = input.just_pressed(KeyCode::Equal) || input.just_pressed(KeyCode::NumpadAdd);
    let zoom_out =
        input.just_pressed(KeyCode::Minus) || input.just_pressed(KeyCode::NumpadSubtract);
    if zoom_in || zoom_out {
        let factor = if zoom_in { 1.15 } else { 1.0 / 1.15 };
        state.zoom = (state.zoom * factor).clamp(0.2, 6.0);
        zoom_changed = true;
    }
    if zoom_changed {
        for (_, mut xf) in &mut roots {
            xf.scale = Vec3::splat(state.zoom);
        }
        if let Some(active) = active.as_ref() {
            set_hud(&mut text, &state, &active.data);
        }
    }

    if state.plain
        && (input.just_pressed(KeyCode::BracketLeft)
            || input.just_pressed(KeyCode::BracketRight)
            || input.just_pressed(KeyCode::Comma)
            || input.just_pressed(KeyCode::Period))
    {
        let len = PlainStyle::ALL.len();
        if input.just_pressed(KeyCode::BracketRight) || input.just_pressed(KeyCode::Period) {
            state.style = (state.style + 1) % len;
        } else {
            state.style = (state.style + len - 1) % len;
        }
        if let Some(active) = active.as_ref() {
            set_hud(&mut text, &state, &active.data);
        }
    }
}

pub fn paint_entity_buttons(
    state: Res<ViewerState>,
    mut picks: Query<(&EntityPick, &Interaction, &mut BackgroundColor)>,
) {
    for (pick, interaction, mut color) in &mut picks {
        *color = if pick.0 == state.entity {
            BackgroundColor(Color::srgb(0.25, 0.48, 0.25))
        } else if *interaction == Interaction::Hovered {
            BackgroundColor(Color::srgb(0.2, 0.32, 0.2))
        } else {
            BackgroundColor(Color::srgb(0.14, 0.18, 0.14))
        };
    }
}

fn load_selection(
    catalog: &EntityCatalog,
    cache: &mut ReanimCache,
    state: &mut ViewerState,
    reset_reanim: bool,
) -> Option<ActiveReanim> {
    let info = catalog.entity(&state.entity)?;
    state.reanim_count = info.reanims.len().max(1);
    if reset_reanim {
        state.reanim_index = info
            .reanims
            .iter()
            .position(|r| r.name == info.name)
            .unwrap_or(0);
    }
    if state.reanim_count == 0 {
        return None;
    }
    state.reanim_index %= state.reanim_count;
    let reanim_name = info.reanims.get(state.reanim_index)?.name.clone();
    state.reanim_name = reanim_name.clone();

    let data = load_cached(cache, catalog, &state.entity, &reanim_name).ok()?;
    // Solo el `Zombie.reanim` base mete cono, cubo, puerta y bandera en el mismo archivo.
    state.plain = state.entity == "Zombie" && reanim_name == "Zombie";
    if !state.plain {
        state.style = 0;
    }
    state.current = pick_initial_anim(&data);

    let flag = if state.plain {
        load_cached(cache, catalog, "Zombie", "Zombie_flagpole").ok()
    } else {
        None
    };
    Some(ActiveReanim { data, flag })
}

fn remember_spawn(state: &mut ViewerState, report: SpawnReport) {
    state.spawned = report.spawned;
    state.skipped = report.skipped;
    state.missing = report.missing;
}

fn spawn_preview(
    commands: &mut Commands,
    asset_server: &AssetServer,
    state: &ViewerState,
    loaded: &ActiveReanim,
) -> SpawnReport {
    let root_y = if state.entity == "Zombie" { 70.0 } else { 10.0 };
    let root = commands
        .spawn((
            Transform::from_xyz(40.0, root_y, 0.0).with_scale(Vec3::splat(state.zoom)),
            Visibility::default(),
            CharacterRoot { speed: 40.0 },
            PreviewRoot,
            TestModeEntity,
        ))
        .id();

    let mut report = spawn_parts(
        commands,
        asset_server,
        &loaded.data,
        &state.entity,
        root,
        true,
        None,
    );
    if state.plain {
        if let (Some(hand), Some(flag)) = (report.flaghand, loaded.flag.as_ref()) {
            attach_flag(commands, asset_server, hand, flag);
        }
    }
    println!(
        "🎭 {} / {} con {} piezas ({} saltadas)",
        state.entity, state.reanim_name, report.spawned, report.skipped
    );
    report.flaghand = None;
    report
}

fn load_cached(
    cache: &mut ReanimCache,
    catalog: &EntityCatalog,
    entity: &str,
    reanim: &str,
) -> Result<ReanimData, String> {
    let key = (entity.to_string(), reanim.to_string());
    if let Some(data) = cache.map.get(&key) {
        return Ok(data.clone());
    }
    let info = catalog
        .entity(entity)
        .ok_or_else(|| format!("❌ No existe la entidad '{entity}'"))?;
    let entry = info
        .reanims
        .iter()
        .find(|r| r.name == reanim)
        .ok_or_else(|| format!("❌ {entity} no tiene el reanim '{reanim}'"))?;
    let mut data = parse_reanim(&entry.path)?;
    if let Err(e) = crate::plant_anim::apply(entity, reanim, &mut data) {
        eprintln!("{e}");
    }
    cache.map.insert(key, data.clone());
    Ok(data)
}

fn pick_initial_anim(data: &ReanimData) -> usize {
    data.states
        .iter()
        .position(|s| {
            matches!(
                s.name.as_str(),
                "anim_walk" | "anim_idle" | "anim_full_idle" | "idle"
            )
        })
        .unwrap_or(0)
}

fn attach_flag(
    commands: &mut Commands,
    asset_server: &AssetServer,
    hand: Entity,
    flag: &ReanimData,
) {
    commands.entity(hand).with_children(|parent| {
        for (idx, track) in flag.tracks.iter().enumerate() {
            let Some(image) = track
                .frames
                .iter()
                .find_map(|frame| frame.as_ref().and_then(|r| r.image.clone()))
            else {
                continue;
            };
            let Some(rel) = image_to_path(&image, "Zombie") else {
                continue;
            };
            if !asset_exists(&rel) {
                continue;
            }
            parent.spawn((
                Sprite {
                    image: asset_server.load(&rel),
                    anchor: bevy::sprite::Anchor::TopLeft,
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, 0.2),
                Visibility::default(),
                FlagPart { track: idx },
            ));
        }
    });
}

fn set_hud(text: &mut Query<&mut Text, With<HudText>>, state: &ViewerState, data: &ReanimData) {
    let s = hud_text(state, data);
    for mut t in text.iter_mut() {
        t.0 = s.clone();
    }
}

pub fn switch_animation(
    input: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ViewerState>,
    active: Option<Res<ActiveReanim>>,
    mut text: Query<&mut Text, With<HudText>>,
    mut next_state: ResMut<NextState<crate::AppState>>,
    back_button: Query<&Interaction, (With<BackButton>, Changed<Interaction>)>,
) {
    for interaction in &back_button {
        if *interaction == Interaction::Pressed {
            next_state.set(crate::AppState::Menu);
            return;
        }
    }

    if input.just_pressed(KeyCode::Escape) {
        next_state.set(crate::AppState::Menu);
        return;
    }

    let Some(active) = active else {
        return;
    };
    let data = &active.data;
    let n = data.states.len();
    if n == 0 {
        return;
    }

    let mut restart = false;
    let mut refresh_hud = false;
    if input.just_pressed(KeyCode::ArrowRight) {
        state.current = (state.current + 1) % n;
        restart = true;
    }
    if input.just_pressed(KeyCode::ArrowLeft) {
        state.current = (state.current + n - 1) % n;
        restart = true;
    }
    if input.just_pressed(KeyCode::Space) {
        restart = true;
    }
    if input.just_pressed(KeyCode::KeyP) {
        state.paused = !state.paused;
        refresh_hud = true;
    }
    if input.just_pressed(KeyCode::KeyM) {
        state.moving = !state.moving;
        refresh_hud = true;
    }

    if restart {
        state.time = 0.0;
    }

    if restart || refresh_hud {
        set_hud(&mut text, &state, data);
    }
}

pub fn play_animation(
    time: Res<Time>,
    mut state: ResMut<ViewerState>,
    active: Option<Res<ActiveReanim>>,
    asset_server: Res<AssetServer>,
    mut parts: Query<
        (
            &mut Transform,
            &mut Sprite,
            &mut Visibility,
            &mut CharacterPart,
        ),
        Without<FlagPart>,
    >,
    mut flags: Query<(&mut Transform, &mut Visibility, &FlagPart), Without<CharacterPart>>,
) {
    if state.skip_pose {
        state.skip_pose = false;
        return;
    }
    let Some(active) = active else {
        return;
    };
    let data = &active.data;

    if !state.paused {
        state.time += time.delta_secs();
    }

    if data.states.is_empty() {
        return;
    }

    let anim = &data.states[state.current % data.states.len()];
    let local = (state.time * data.fps) as usize % anim.len.max(1);
    let blink = anim.blink_at(state.time, data.fps, 0.0);
    let style = if state.plain {
        Some(PlainStyle::ALL[state.style % PlainStyle::ALL.len()])
    } else {
        None
    };

    for (mut xf, mut sprite, mut vis, mut part) in &mut parts {
        if part.track >= data.tracks.len() {
            *vis = Visibility::Hidden;
            continue;
        }
        pose_part(
            data,
            anim,
            local,
            &state.entity,
            style,
            &asset_server,
            &mut xf,
            &mut sprite,
            &mut vis,
            &mut part,
            blink,
        );
    }

    let show_flag = style == Some(PlainStyle::Flag);
    if let Some(flag) = &active.flag {
        let frame = (state.time * flag.fps) as usize;
        for (mut xf, mut vis, part) in &mut flags {
            if !show_flag {
                *vis = Visibility::Hidden;
                continue;
            }
            let Some(track) = flag.tracks.get(part.track) else {
                *vis = Visibility::Hidden;
                continue;
            };
            let len = track.frames.len().max(1);
            match track.frames[frame % len].as_ref() {
                Some(r) if r.active => {
                    *vis = Visibility::Inherited;
                    xf.translation = Vec3::new(r.x, -r.y, 0.2);
                    xf.scale = Vec3::new(r.sx, r.sy, 1.0);
                    xf.rotation = Quat::from_rotation_z(-r.kx.to_radians());
                }
                _ => *vis = Visibility::Hidden,
            }
        }
    }
}
