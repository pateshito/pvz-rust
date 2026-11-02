use crate::assets::paths;
use crate::reanim::ReanimData;
use crate::resources::ViewerState;
use crate::Character;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Nombre de la carpeta de la entidad en `assets/animations/` y
/// `assets/textures/`. Sustituye a las tablas por personaje.
pub fn entity_name(character: Character) -> &'static str {
    match character {
        Character::Zombie => "Zombie",
        Character::PeaShooter => "PeaShooter",
    }
}

/// Resuelve un id `IMAGE_REANIM_*` con el índice genérico de assets.
pub fn image_to_path(internal: &str, character: Character) -> Option<String> {
    paths::resolve_image(internal, entity_name(character))
}

/// Carpeta `assets/`, resuelta en tiempo de ejecución con la misma lógica que usa
/// Bevy: `BEVY_ASSET_ROOT`, luego `CARGO_MANIFEST_DIR` (con `cargo run`) y, por
/// último, la carpeta del ejecutable. Así el binario sigue funcionando si se mueve.
pub fn assets_dir() -> &'static Path {
    static ASSETS_DIR: OnceLock<PathBuf> = OnceLock::new();
    ASSETS_DIR.get_or_init(|| {
        let base = if let Ok(root) = std::env::var("BEVY_ASSET_ROOT") {
            PathBuf::from(root)
        } else if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
            PathBuf::from(manifest)
        } else {
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(Path::to_path_buf))
                .unwrap_or_else(|| PathBuf::from("."))
        };
        base.join("assets")
    })
}

pub fn asset_exists(rel: &str) -> bool {
    assets_dir().join(rel).exists()
}

pub fn hud_text(state: &ViewerState, data: &ReanimData) -> String {
    if data.states.is_empty() {
        return format!(
            "{:?} - sin animaciones\n←/→: N/A | Space | P | M | Esc",
            state.character
        );
    }
    let a = &data.states[state.current % data.states.len()];
    format!(
        "[{}/{}] {:?} - {}\n←/→: cambiar | Space: reiniciar | P: pausa{} | M: mover{} | Esc: menú",
        state.current % data.states.len() + 1,
        data.states.len(),
        state.character,
        a.name,
        if state.paused { " [PAUSADO]" } else { "" },
        if state.moving { " [MOVIMIENTO]" } else { "" },
    )
}
