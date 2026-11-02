use crate::components::{BackButton, MenuAction, MenuButton};
use crate::AppState;
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct MenuEntity;

pub fn setup_menu(mut commands: Commands) {
    commands.spawn((Camera2d, MenuEntity));

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.15, 0.1)),
            MenuEntity,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("🌻 PVZ Reanim Viewer 🧟"),
                TextFont::from_font_size(40.0),
                TextColor(Color::srgb(0.9, 0.9, 0.3)),
            ));

            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(300.0),
                        height: Val::Px(60.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor(Color::srgb(0.3, 0.6, 0.3)),
                    BackgroundColor(Color::srgb(0.2, 0.4, 0.2)),
                    MenuButton,
                    MenuAction::Play,
                    MenuEntity,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Jugar"),
                        TextFont::from_font_size(24.0),
                        TextColor(Color::WHITE),
                    ));
                });

            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(300.0),
                        height: Val::Px(60.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor(Color::srgb(0.3, 0.6, 0.3)),
                    BackgroundColor(Color::srgb(0.2, 0.4, 0.2)),
                    MenuButton,
                    MenuAction::Test,
                    MenuEntity,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Test"),
                        TextFont::from_font_size(24.0),
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

pub fn menu_interaction(
    mut next_state: ResMut<NextState<AppState>>,
    mut interaction_query: Query<
        (&Interaction, &MenuAction, &mut BackgroundColor),
        (Changed<Interaction>, With<MenuButton>),
    >,
) {
    for (interaction, action, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                println!("🎮 Botón presionado: {:?}", action);
                next_state.set(match action {
                    MenuAction::Play => AppState::Play,
                    MenuAction::Test => AppState::TestMode,
                });
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.3, 0.6, 0.3));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::srgb(0.2, 0.4, 0.2));
            }
        }
    }
}

pub fn cleanup_menu(mut commands: Commands, query: Query<Entity, With<MenuEntity>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    println!("🧹 Menú limpiado");
}

#[derive(Component)]
pub(crate) struct PlayEntity;

/// La simulación de la partida existe, pero el tablero todavía no se dibuja.
pub fn setup_play(mut commands: Commands) {
    commands.spawn((Camera2d, PlayEntity));
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.18, 0.1)),
            PlayEntity,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Jugar"),
                TextFont::from_font_size(40.0),
                TextColor(Color::srgb(0.9, 0.9, 0.3)),
            ));
            parent.spawn((
                Text::new("El tablero todavía no está en pantalla."),
                TextFont::from_font_size(20.0),
                TextColor(Color::srgb(0.85, 0.85, 0.85)),
            ));
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(160.0),
                        height: Val::Px(44.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BorderColor(Color::srgb(0.6, 0.3, 0.3)),
                    BackgroundColor(Color::srgb(0.4, 0.2, 0.2)),
                    BackButton,
                    PlayEntity,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("← Menú"),
                        TextFont::from_font_size(18.0),
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

pub fn play_input(
    mut next_state: ResMut<NextState<AppState>>,
    input: Res<ButtonInput<KeyCode>>,
    back_button: Query<&Interaction, (With<BackButton>, Changed<Interaction>)>,
) {
    for interaction in &back_button {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Menu);
            return;
        }
    }
    if input.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::Menu);
    }
}

pub fn cleanup_play(mut commands: Commands, query: Query<Entity, With<PlayEntity>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
