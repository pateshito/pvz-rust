//! Selector de niveles: se abre al pulsar «Jugar» y desbloquea el siguiente
//! nivel cuando se gana el anterior.

use bevy::prelude::*;

use crate::game::level::{Level, LEVEL_COUNT};
use crate::resources::LevelProgress;
use crate::AppState;

#[derive(Component)]
pub(crate) struct LevelSelectEntity;

#[derive(Component)]
pub(crate) struct LevelButton(usize);

#[derive(Component)]
pub(crate) struct LevelNotice;

pub fn setup_level_select(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    progress: Res<LevelProgress>,
) {
    commands.spawn((Camera2d, LevelSelectEntity));
    commands.spawn((
        Sprite {
            image: asset_server.load("images/background1.jpg"),
            custom_size: Some(Vec2::new(1400.0, 600.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -10.0),
        LevelSelectEntity,
    ));

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(18.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
            LevelSelectEntity,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("Selecciona un nivel"),
                TextFont::from_font_size(38.0),
                TextColor(Color::srgb(0.98, 0.95, 0.75)),
            ));
            root.spawn((
                Text::new(format!(
                    "{} de {} niveles completados",
                    progress.cleared.min(LEVEL_COUNT),
                    LEVEL_COUNT
                )),
                TextFont::from_font_size(17.0),
                TextColor(Color::srgb(0.85, 0.88, 0.72)),
            ));

            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(14.0),
                ..default()
            })
            .with_children(|row| {
                for index in 0..LEVEL_COUNT {
                    let level = Level::campaign(index);
                    let unlocked = progress.is_unlocked(index);
                    let cleared = progress.is_cleared(index);
                    let background = card_color(unlocked, cleared, false);

                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(130.0),
                            height: Val::Px(150.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            row_gap: Val::Px(7.0),
                            border: UiRect::all(Val::Px(3.0)),
                            ..default()
                        },
                        BorderColor(border_color(unlocked, cleared)),
                        BackgroundColor(background),
                        LevelButton(index),
                    ))
                    .with_children(|card| {
                        card.spawn((
                            Text::new(format!("Nivel {}", index + 1)),
                            TextFont::from_font_size(20.0),
                            TextColor(Color::WHITE),
                        ));
                        card.spawn((
                            Text::new(short_name(level.name)),
                            TextFont::from_font_size(11.0),
                            TextColor(Color::srgb(0.88, 0.90, 0.80)),
                            TextLayout::new_with_justify(JustifyText::Center),
                        ));
                        card.spawn((
                            Text::new(format!("{} zombis", level.total_zombies())),
                            TextFont::from_font_size(12.0),
                            TextColor(Color::srgb(0.78, 0.82, 0.74)),
                        ));
                        card.spawn((
                            Text::new(status(unlocked, cleared)),
                            TextFont::from_font_size(13.0),
                            TextColor(if unlocked {
                                Color::srgb(0.98, 0.95, 0.75)
                            } else {
                                Color::srgb(0.65, 0.65, 0.65)
                            }),
                        ));
                    });
                }
            });

            root.spawn((
                Text::new(""),
                TextFont::from_font_size(14.0),
                TextColor(Color::srgb(0.98, 0.80, 0.55)),
                LevelNotice,
            ));
            root.spawn((
                Text::new("Esc: volver al menú · Gana un nivel para desbloquear el siguiente"),
                TextFont::from_font_size(13.0),
                TextColor(Color::srgb(0.75, 0.80, 0.70)),
            ));
        });
}

fn short_name(name: &str) -> &str {
    name.split('·').nth(1).map(str::trim).unwrap_or(name)
}

fn status(unlocked: bool, cleared: bool) -> &'static str {
    if cleared {
        "Completado"
    } else if unlocked {
        "Jugar"
    } else {
        "Bloqueado"
    }
}

fn card_color(unlocked: bool, cleared: bool, hovered: bool) -> Color {
    if !unlocked {
        Color::srgb(0.14, 0.15, 0.14)
    } else if hovered {
        Color::srgb(0.26, 0.55, 0.24)
    } else if cleared {
        Color::srgb(0.22, 0.34, 0.16)
    } else {
        Color::srgb(0.18, 0.42, 0.18)
    }
}

fn border_color(unlocked: bool, cleared: bool) -> Color {
    if !unlocked {
        Color::srgb(0.25, 0.26, 0.24)
    } else if cleared {
        Color::srgb(0.85, 0.75, 0.30)
    } else {
        Color::srgb(0.35, 0.65, 0.30)
    }
}

pub fn level_select_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::Menu);
    }
}

pub fn level_select_interaction(
    mut next_state: ResMut<NextState<AppState>>,
    mut progress: ResMut<LevelProgress>,
    mut notice: Query<&mut Text, With<LevelNotice>>,
    mut cards: Query<
        (&Interaction, &LevelButton, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, button, mut background) in &mut cards {
        let index = button.0;
        let unlocked = progress.is_unlocked(index);
        let cleared = progress.is_cleared(index);
        match *interaction {
            Interaction::Pressed => {
                if unlocked {
                    progress.selected = index;
                    next_state.set(AppState::Play);
                } else if let Ok(mut text) = notice.get_single_mut() {
                    text.0 = format!(
                        "Termina el nivel {} para desbloquear el nivel {}.",
                        index,
                        index + 1
                    );
                }
            }
            Interaction::Hovered => {
                *background = BackgroundColor(card_color(unlocked, cleared, unlocked));
            }
            Interaction::None => {
                *background = BackgroundColor(card_color(unlocked, cleared, false));
            }
        }
    }
}

pub fn cleanup_level_select(mut commands: Commands, query: Query<Entity, With<LevelSelectEntity>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}
