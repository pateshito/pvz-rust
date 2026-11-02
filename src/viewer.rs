use crate::character::{pose_part, spawn_parts};
use crate::components::{BackButton, CharacterPart, CharacterRoot, HudText};
use crate::resources::{CharacterLibraries, SelectedCharacter, ViewerState};
use crate::utils::hud_text;
use crate::Character;
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct TestModeEntity;

pub fn setup_test_mode(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    libraries: Res<CharacterLibraries>,
    selected: Res<SelectedCharacter>,
) {
    let character = selected.0;
    let data = match character {
        Character::Zombie => &libraries.zombie,
        Character::PeaShooter => &libraries.peashooter,
    };

    commands.spawn((Camera2d, TestModeEntity));

    let initial_anim = data
        .states
        .iter()
        .position(|s| {
            matches!(
                s.name.as_str(),
                "anim_walk" | "anim_idle" | "anim_full_idle" | "idle"
            )
        })
        .unwrap_or(0);

    let initial_state = ViewerState {
        current: initial_anim,
        time: 0.0,
        paused: false,
        moving: false,
        character,
    };

    commands.spawn((
        Text::new(hud_text(&initial_state, data)),
        TextFont::from_font_size(18.0),
        TextColor(Color::srgb(0.9, 0.9, 0.9)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(10.0),
            ..default()
        },
        HudText,
        TestModeEntity,
    ));

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

    let root_y = match character {
        Character::Zombie => 65.0,
        Character::PeaShooter => 0.0,
    };

    let root = commands
        .spawn((
            Transform::from_xyz(0.0, root_y, 0.0),
            Visibility::default(),
            CharacterRoot { speed: 40.0 },
            TestModeEntity,
        ))
        .id();

    let (count, skipped) = spawn_parts(&mut commands, &asset_server, data, character, root, true);

    println!(
        "🎭 {:?} construido con {} partes visibles ({} saltadas)",
        character, count, skipped
    );

    commands.insert_resource(initial_state);
}

pub fn cleanup_test_mode(mut commands: Commands, query: Query<Entity, With<TestModeEntity>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    println!("🧹 Test mode limpiado");
}

pub fn switch_animation(
    input: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ViewerState>,
    libraries: Res<CharacterLibraries>,
    mut text: Query<&mut Text, With<HudText>>,
    mut next_state: ResMut<NextState<crate::AppState>>,
    back_button: Query<&Interaction, (With<BackButton>, Changed<Interaction>)>,
) {
    let data = match state.character {
        Character::Zombie => &libraries.zombie,
        Character::PeaShooter => &libraries.peashooter,
    };

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

    let n = data.states.len();
    if n == 0 {
        return;
    }

    // `restart`: cambiar de animación o pulsar Space reinicia el tiempo.
    // `refresh_hud`: pausa y movimiento solo actualizan el texto, sin tocar el tiempo.
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
        let s = hud_text(&state, data);
        for mut t in &mut text {
            t.0 = s.clone();
        }
    }
}

pub fn play_animation(
    time: Res<Time>,
    mut state: ResMut<ViewerState>,
    libraries: Res<CharacterLibraries>,
    asset_server: Res<AssetServer>,
    mut parts: Query<(
        &mut Transform,
        &mut Sprite,
        &mut Visibility,
        &mut CharacterPart,
    )>,
) {
    let data = match state.character {
        Character::Zombie => &libraries.zombie,
        Character::PeaShooter => &libraries.peashooter,
    };

    if !state.paused {
        state.time += time.delta_secs();
    }

    if data.states.is_empty() {
        return;
    }

    let anim = &data.states[state.current % data.states.len()];
    // Frame dentro del bucle del estado (0..len). Cada grupo de piezas lo
    // traduce a su propio frame de origen.
    let local = (state.time * data.fps) as usize % anim.len.max(1);

    for (mut xf, mut sprite, mut vis, mut part) in &mut parts {
        pose_part(
            data,
            anim,
            local,
            state.character,
            &asset_server,
            &mut xf,
            &mut sprite,
            &mut vis,
            &mut part,
        );
    }
}
