use crate::components::CharacterRoot;
use crate::resources::ViewerState;
use bevy::prelude::*;

pub fn move_characters(
    mut query: Query<(&mut Transform, &CharacterRoot)>,
    time: Res<Time>,
    state: Res<ViewerState>,
) {
    if !state.moving {
        return;
    }
    for (mut transform, _) in &mut query {
        transform.translation.x -= 40.0 * time.delta_secs();
        if transform.translation.x < -500.0 {
            transform.translation.x = 500.0;
        }
    }
}
