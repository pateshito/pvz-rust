use bevy::prelude::*;

#[derive(Component)]
pub struct CharacterRoot {
    pub speed: f32,
}

#[derive(Component)]
pub struct CharacterPart {
    pub track: usize,
    pub current_image: Option<String>,
    /// Ejes de la pieza en este frame (ver `Resolved::axes`). Si traen sesgo, un
    /// `Transform` no los puede representar: `character::apply_part_shear` los
    /// escribe en el `GlobalTransform` después de que Bevy lo calcula.
    pub axes: Option<Mat2>,
}

#[derive(Component)]
pub struct MenuButton;

#[derive(Component, Debug)]
pub enum MenuAction {
    Play,
    Plants,
    Test,
}

/// Botón de la lista de entidades del modo test.
#[derive(Component)]
pub struct EntityPick(pub String);

/// Columna con scroll de la lista de entidades.
#[derive(Component)]
pub struct EntityList;

/// Raíz del personaje que se está previsualizando (se recrea al cambiar).
#[derive(Component)]
pub struct PreviewRoot;

#[derive(Component)]
pub struct BackButton;

#[derive(Component)]
pub struct HudText;
