use crate::components::{BackButton, CharacterPart, CharacterRoot, HudText};
use crate::resources::{CharacterLibraries, SelectedCharacter, ViewerState};
use crate::utils::{asset_exists, get_allowlist, hud_text, image_to_path};
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
        .animations
        .iter()
        .position(|a| a.name == "anim_walk" || a.name == "anim_idle" || a.name == "anim_full_idle")
        .unwrap_or(0);

    commands.spawn((
        Text::new(""),
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

    let allowlist = get_allowlist(character);
    let mut count = 0;
    let mut skipped = 0;

    for (idx, track) in data.tracks.iter().enumerate() {
        if !allowlist.contains(&track.name.as_str()) {
            continue;
        }

        let first_image = match track
            .frames
            .iter()
            .find_map(|r| r.as_ref().and_then(|r| r.image.clone()))
        {
            Some(i) => i,
            None => {
                println!("⚠️  Track '{}' sin imagen en ningún frame", track.name);
                skipped += 1;
                continue;
            }
        };

        let rel = match image_to_path(&first_image, character) {
            Some(p) => p,
            None => {
                println!("⚠️  Sin mapeo para: {} (track {})", first_image, track.name);
                skipped += 1;
                continue;
            }
        };

        if !asset_exists(&rel) {
            println!("⚠️  Falta asset: {} (track {})", rel, track.name);
            skipped += 1;
            continue;
        }

        let z = idx as f32 * 0.001;
        commands.entity(root).with_children(|parent| {
            parent.spawn((
                Sprite {
                    image: asset_server.load(&rel),
                    anchor: bevy::sprite::Anchor::TopLeft,
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, z),
                Visibility::default(),
                CharacterPart {
                    track: idx,
                    current_image: Some(first_image.clone()),
                },
                TestModeEntity,
            ));
        });
        count += 1;
    }

    println!(
        "🎭 {:?} construido con {} partes visibles ({} saltadas)",
        character, count, skipped
    );

    commands.insert_resource(ViewerState {
        current: initial_anim,
        time: 0.0,
        paused: false,
        moving: false,
        character,
    });
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

    let n = data.animations.len();
    if n == 0 {
        return;
    }

    let mut dirty = false;
    if input.just_pressed(KeyCode::ArrowRight) {
        state.current = (state.current + 1) % n;
        dirty = true;
    }
    if input.just_pressed(KeyCode::ArrowLeft) {
        state.current = (state.current + n - 1) % n;
        dirty = true;
    }
    if input.just_pressed(KeyCode::Space) {
        state.time = 0.0;
    }
    if input.just_pressed(KeyCode::KeyP) {
        state.paused = !state.paused;
        dirty = true;
    }
    if input.just_pressed(KeyCode::KeyM) {
        state.moving = !state.moving;
        dirty = true;
    }

    if dirty {
        state.time = 0.0;
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

    if data.animations.is_empty() {
        return;
    }

    let anim = &data.animations[state.current % data.animations.len()];
    let len = anim.end.saturating_sub(anim.start) + 1;
    let frame = anim.start + ((state.time * data.fps) as usize % len);

    for (mut xf, mut sprite, mut vis, mut part) in &mut parts {
        let track = &data.tracks[part.track];
        let is_prop = track.name.starts_with("anim_");

        match track.frames.get(frame).and_then(|r| r.as_ref()) {
            Some(r) => {
                if !r.active {
                    *vis = Visibility::Hidden;
                    continue;
                }
                *vis = Visibility::Inherited;
                xf.translation = Vec3::new(r.x, -r.y, xf.translation.z);
                xf.scale = Vec3::new(r.sx, r.sy, 1.0);
                xf.rotation = Quat::from_rotation_z(-r.kx.to_radians());

                // Cambiar imagen si el track usa múltiples sprites (ej: blink1/blink2)
                if let Some(ref new_img) = r.image {
                    if part.current_image.as_deref() != Some(new_img.as_str()) {
                        if let Some(rel) = image_to_path(new_img, state.character) {
                            if asset_exists(&rel) {
                                sprite.image = asset_server.load(&rel);
                                part.current_image = Some(new_img.clone());
                            }
                        }
                    }
                }
            }
            None => {
                if is_prop {
                    *vis = Visibility::Hidden;
                }
            }
        }
    }
}
