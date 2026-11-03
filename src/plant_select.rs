//! Selector de plantas. Arriba replica el banco de semillas del HUD con las
//! plantas equipadas; abajo quedan las no elegidas. Las bloqueadas se dibujan
//! con su modelo real en silueta negra. Un clic explica cómo desbloquearlas.

use bevy::prelude::*;
use bevy::render::view::RenderLayers;

use crate::components::CharacterPart;
use crate::game::defs::{plant_def, plant_unlock_level, PlantKind, ALL_PLANTS};
use crate::game::view::{self, PREVIEW_LAYER};
use crate::resources::{CharacterLibraries, LevelProgress, PlantLoadout};
use crate::AppState;

/// Mismas medidas que el banco de semillas de `game::view`.
const BANK_X: f32 = 8.0;
const BANK_Y: f32 = 4.0;
const BANK_W: f32 = 446.0;
const BANK_H: f32 = 87.0;
const PACKET_X: f32 = 83.0;
const PACKET_Y: f32 = 8.0;
const PACKET_W: f32 = 50.0;
const PACKET_H: f32 = 70.0;
const PACKET_STEP: f32 = 52.0;
const ICON_INSET_X: f32 = 5.0;
const ICON_TOP: f32 = 4.0;
const ICON_BOTTOM: f32 = 16.0;

const CARD_W: f32 = 145.0;
const CARD_H: f32 = 235.0;
const CARD_GAP: f32 = 14.0;
const CARD_TOP: f32 = 205.0;
const ICON_CARD_TOP: f32 = 14.0;
const ICON_CARD_BOTTOM: f32 = 154.0;
const ICON_CARD_INSET: f32 = 16.0;

#[derive(Component)]
pub(crate) struct PlantSelectEntity;

#[derive(Component)]
pub(crate) struct PlantCard(PlantKind);

#[derive(Component)]
pub(crate) struct PlantStatusText(PlantKind);

#[derive(Component)]
pub(crate) struct EquippedSlot(usize);

#[derive(Component)]
pub(crate) struct PlantNotice;

/// Modelo real de una planta. `locked` lo pinta entero de negro.
#[derive(Component)]
pub(crate) struct PlantIcon {
    kind: PlantKind,
    locked: bool,
}

pub fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    libraries: Res<CharacterLibraries>,
    progress: Res<LevelProgress>,
    loadout: Res<PlantLoadout>,
) {
    spawn_screen(
        &mut commands,
        &asset_server,
        &libraries,
        &progress,
        &loadout,
    );
}

fn spawn_screen(
    commands: &mut Commands,
    asset_server: &AssetServer,
    libraries: &CharacterLibraries,
    progress: &LevelProgress,
    loadout: &PlantLoadout,
) {
    commands.spawn((Camera2d, IsDefaultUiCamera, PlantSelectEntity));
    // Los modelos de planta se dibujan encima de la UI, igual que los sobres del HUD.
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::layer(PREVIEW_LAYER),
        PlantSelectEntity,
    ));
    commands.spawn((
        Sprite {
            image: asset_server.load("images/background1.jpg"),
            custom_size: Some(Vec2::new(1400.0, 600.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -10.0),
        PlantSelectEntity,
    ));

    let unlocked = ALL_PLANTS
        .iter()
        .filter(|&&kind| progress.plant_unlocked(kind))
        .count();
    let available: Vec<PlantKind> = ALL_PLANTS
        .into_iter()
        .filter(|kind| !loadout.chosen.contains(kind))
        .collect();

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.38)),
            PlantSelectEntity,
        ))
        .with_children(|root| {
            spawn_bank(root, asset_server, loadout);
            spawn_header(root, unlocked, loadout);
            spawn_available_cards(root, &available);
            spawn_footer(root, available.is_empty());
        });

    // Modelos equipados, en la misma caja que los sobres del HUD.
    for (slot, kind) in loadout.chosen.iter().copied().enumerate() {
        let entity = view::spawn_plant_icon(
            commands,
            asset_server,
            libraries,
            kind,
            hud_icon_box(slot),
            20.0,
        );
        commands.entity(entity).insert((
            PlantSelectEntity,
            PlantIcon {
                kind,
                locked: false,
            },
        ));
    }

    // Modelos de las plantas no elegidas, dentro de su tarjeta.
    let start_x = (800.0
        - (available.len() as f32 * CARD_W
            + (available.len().saturating_sub(1)) as f32 * CARD_GAP))
        * 0.5;
    for (index, kind) in available.into_iter().enumerate() {
        let x = start_x + index as f32 * (CARD_W + CARD_GAP);
        let target = (
            x + ICON_CARD_INSET,
            CARD_TOP + ICON_CARD_TOP,
            x + CARD_W - ICON_CARD_INSET,
            CARD_TOP + ICON_CARD_BOTTOM,
        );
        let entity = view::spawn_plant_icon(commands, asset_server, libraries, kind, target, 20.0);
        commands.entity(entity).insert((
            PlantSelectEntity,
            PlantIcon {
                kind,
                locked: !progress.plant_unlocked(kind),
            },
        ));
    }
}

fn spawn_bank(root: &mut ChildBuilder, asset_server: &AssetServer, loadout: &PlantLoadout) {
    root.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(BANK_X),
            top: Val::Px(BANK_Y),
            width: Val::Px(BANK_W),
            height: Val::Px(BANK_H),
            ..default()
        },
        ImageNode::new(asset_server.load("images/SeedBank.png")),
    ))
    .with_children(|bank| {
        for slot in 0..ALL_PLANTS.len() {
            let equipped = slot < loadout.chosen.len();
            bank.spawn((
                Button,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(PACKET_X + PACKET_STEP * slot as f32),
                    top: Val::Px(PACKET_Y),
                    width: Val::Px(PACKET_W),
                    height: Val::Px(PACKET_H),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                ImageNode {
                    image: asset_server.load("images/SeedPacket_Larger.png"),
                    color: if equipped {
                        Color::WHITE
                    } else {
                        Color::srgba(0.35, 0.35, 0.35, 0.55)
                    },
                    ..default()
                },
                BorderColor(if equipped {
                    Color::srgb(0.88, 0.78, 0.32)
                } else {
                    Color::srgba(0.0, 0.0, 0.0, 0.0)
                }),
                EquippedSlot(slot),
            ));
        }
    });
}

fn spawn_header(root: &mut ChildBuilder, unlocked: usize, loadout: &PlantLoadout) {
    root.spawn((
        Text::new("Elige tus plantas"),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(103.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        TextFont::from_font_size(34.0),
        TextColor(Color::srgb(0.98, 0.95, 0.75)),
        TextLayout::new_with_justify(JustifyText::Center),
    ));
    root.spawn((
        Text::new(format!(
            "Arriba: equipadas ({}/5) · Abajo: no elegidas · Desbloqueadas: {unlocked}/{}",
            loadout.chosen.len(),
            ALL_PLANTS.len()
        )),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(147.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        TextFont::from_font_size(15.0),
        TextColor(Color::srgb(0.85, 0.88, 0.72)),
        TextLayout::new_with_justify(JustifyText::Center),
    ));
    root.spawn((
        Text::new("Equipadas\nClic en un sobre para quitar"),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(470.0),
            top: Val::Px(18.0),
            ..default()
        },
        TextFont::from_font_size(14.0),
        TextColor(Color::srgb(0.90, 0.92, 0.80)),
    ));
}

fn spawn_available_cards(root: &mut ChildBuilder, available: &[PlantKind]) {
    if available.is_empty() {
        root.spawn((
            Text::new("Todas las plantas están equipadas."),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(300.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            TextFont::from_font_size(18.0),
            TextColor(Color::srgb(0.88, 0.90, 0.75)),
            TextLayout::new_with_justify(JustifyText::Center),
        ));
        return;
    }

    let total =
        available.len() as f32 * CARD_W + (available.len().saturating_sub(1)) as f32 * CARD_GAP;
    let start_x = (800.0 - total) * 0.5;

    for (index, &kind) in available.iter().enumerate() {
        let x = start_x + index as f32 * (CARD_W + CARD_GAP);
        let def = plant_def(kind);
        root.spawn((
            Button,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(x),
                top: Val::Px(CARD_TOP),
                width: Val::Px(CARD_W),
                height: Val::Px(CARD_H),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexEnd,
                padding: UiRect::bottom(Val::Px(12.0)),
                row_gap: Val::Px(4.0),
                border: UiRect::all(Val::Px(3.0)),
                ..default()
            },
            BorderColor(Color::srgb(0.28, 0.52, 0.25)),
            BackgroundColor(Color::srgba(0.12, 0.24, 0.12, 0.82)),
            PlantCard(kind),
        ))
        .with_children(|card| {
            card.spawn((
                Text::new(def.name),
                TextFont::from_font_size(14.0),
                TextColor(Color::WHITE),
                TextLayout::new_with_justify(JustifyText::Center),
            ));
            card.spawn((
                Text::new("Disponible"),
                TextFont::from_font_size(12.0),
                TextColor(Color::srgb(0.98, 0.95, 0.75)),
                PlantStatusText(kind),
            ));
        });
    }
}

fn spawn_footer(root: &mut ChildBuilder, all_equipped: bool) {
    root.spawn((
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(472.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        TextFont::from_font_size(15.0),
        TextColor(Color::srgb(0.98, 0.80, 0.55)),
        TextLayout::new_with_justify(JustifyText::Center),
        PlantNotice,
    ));
    root.spawn((
        Text::new(if all_equipped {
            "Esc: volver al menú"
        } else {
            "Clic: añadir o quitar planta · Esc: volver al menú"
        }),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(528.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        TextFont::from_font_size(13.0),
        TextColor(Color::srgb(0.75, 0.80, 0.70)),
        TextLayout::new_with_justify(JustifyText::Center),
    ));
}

fn hud_icon_box(slot: usize) -> (f32, f32, f32, f32) {
    let min_x = BANK_X + PACKET_X + PACKET_STEP * slot as f32 + ICON_INSET_X;
    let min_y = BANK_Y + PACKET_Y + ICON_TOP;
    (
        min_x,
        min_y,
        min_x + PACKET_W - ICON_INSET_X * 2.0,
        min_y + PACKET_H - ICON_TOP - ICON_BOTTOM,
    )
}

pub fn plant_select_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::Menu);
    }
}

pub fn plant_select_interaction(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    libraries: Res<CharacterLibraries>,
    progress: Res<LevelProgress>,
    mut loadout: ResMut<PlantLoadout>,
    mut notice: Query<&mut Text, With<PlantNotice>>,
    mut cards: Query<
        (&Interaction, &PlantCard),
        (Changed<Interaction>, With<Button>, Without<EquippedSlot>),
    >,
    mut slots: Query<
        (&Interaction, &EquippedSlot),
        (Changed<Interaction>, With<Button>, Without<PlantCard>),
    >,
    screen: Query<Entity, With<PlantSelectEntity>>,
) {
    let mut dirty = false;

    for (interaction, card) in &mut cards {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let kind = card.0;
        let def = plant_def(kind);

        if !progress.plant_unlocked(kind) {
            if let Ok(mut text) = notice.get_single_mut() {
                let level = plant_unlock_level(kind).unwrap_or(0) + 1;
                text.0 = format!("Completa el Nivel {level} para desbloquear: {}.", def.name);
            }
            continue;
        }

        if let Err(message) = loadout.toggle(kind) {
            if let Ok(mut text) = notice.get_single_mut() {
                text.0 = message.to_string();
            }
        } else {
            dirty = true;
        }
    }

    for (interaction, slot) in &mut slots {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Some(kind) = loadout.chosen.get(slot.0).copied() else {
            if let Ok(mut text) = notice.get_single_mut() {
                text.0 = "Ese espacio está vacío.".into();
            }
            continue;
        };
        if let Err(message) = loadout.toggle(kind) {
            if let Ok(mut text) = notice.get_single_mut() {
                text.0 = message.to_string();
            }
        } else {
            dirty = true;
        }
    }

    if dirty {
        for entity in &screen {
            commands.entity(entity).despawn_recursive();
        }
        spawn_screen(
            &mut commands,
            &asset_server,
            &libraries,
            &progress,
            &loadout,
        );
    }
}

pub fn paint_cards(
    progress: Res<LevelProgress>,
    mut cards: Query<
        (
            &PlantCard,
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<Button>,
    >,
    mut statuses: Query<(&PlantStatusText, &mut Text)>,
) {
    for (card, interaction, mut background, mut border) in &mut cards {
        let kind = card.0;
        let unlocked = progress.plant_unlocked(kind);
        let hovered = *interaction == Interaction::Hovered;
        *background = BackgroundColor(if !unlocked {
            Color::srgba(0.10, 0.11, 0.10, 0.86)
        } else if hovered {
            Color::srgba(0.18, 0.38, 0.18, 0.92)
        } else {
            Color::srgba(0.12, 0.24, 0.12, 0.82)
        });
        *border = BorderColor(if unlocked {
            Color::srgb(0.34, 0.62, 0.30)
        } else {
            Color::srgb(0.24, 0.25, 0.23)
        });
    }

    for (status_text, mut text) in &mut statuses {
        let kind = status_text.0;
        text.0 = if progress.plant_unlocked(kind) {
            "Disponible".into()
        } else {
            "Bloqueada".into()
        };
    }
}

pub fn pose_icons(
    libraries: Res<CharacterLibraries>,
    asset_server: Res<AssetServer>,
    mut parts: Query<(
        &mut Transform,
        &mut Sprite,
        &mut Visibility,
        &mut CharacterPart,
    )>,
    icons: Query<(&PlantIcon, &Children)>,
) {
    for (icon, children) in &icons {
        view::pose_plant_icon(&mut parts, children, &libraries, icon.kind, &asset_server);
        let tint = if icon.locked {
            Color::srgb(0.02, 0.02, 0.02)
        } else {
            Color::WHITE
        };
        let mut iter = parts.iter_many_mut(children);
        while let Some((_, mut sprite, _, _)) = iter.fetch_next() {
            sprite.color = tint;
        }
    }
}

pub fn cleanup(mut commands: Commands, query: Query<Entity, With<PlantSelectEntity>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}
