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
/// `entity` es el nombre de la carpeta en `assets/animations/`.
pub fn image_to_path(internal: &str, entity: &str) -> Option<String> {
    paths::resolve_image(internal, entity)
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
    let style = if state.plain {
        let styles = crate::zombie_style::PlainStyle::ALL;
        let style = styles[state.style % styles.len()];
        format!(
            "estilo {}/{}: {}   [ ] o , .\n",
            state.style % styles.len() + 1,
            styles.len(),
            style.label()
        )
    } else {
        String::new()
    };
    let reanim = if state.reanim_count > 1 {
        format!(
            "archivo {}/{}: {}   RePág/AvPág\n",
            state.reanim_index + 1,
            state.reanim_count,
            state.reanim_name
        )
    } else {
        String::new()
    };
    let pieces = if state.skipped == 0 {
        format!("{} piezas", state.spawned)
    } else {
        format!("{} piezas, {} sin asset", state.spawned, state.skipped)
    };
    let stats = format!(
        "{:.0} fps · {} tracks · {pieces} · zoom {:.0}%\n",
        data.fps,
        data.tracks.len(),
        state.zoom * 100.0
    );
    let missing = if state.missing.is_empty() {
        String::new()
    } else {
        let shown = state.missing.len().min(4);
        let names = state.missing[..shown].join(", ");
        let extra = state.missing.len() - shown;
        if extra > 0 {
            format!("faltan: {names} (+{extra})\n")
        } else {
            format!("faltan: {names}\n")
        }
    };
    let controls = format!(
        "↑/↓: entidad | ←/→: animación | rueda o +/-: zoom | Space: reiniciar | P: pausa{} | M: mover{} | Esc: menú",
        if state.paused { " [PAUSADO]" } else { "" },
        if state.moving { " [MOVIMIENTO]" } else { "" },
    );
    if data.states.is_empty() {
        return format!(
            "{}\n{reanim}{stats}{missing}{style}sin animaciones\n{controls}",
            state.entity
        );
    }
    let a = &data.states[state.current % data.states.len()];
    format!(
        "{}\n{reanim}{stats}{missing}{style}[{}/{}] {}\n{controls}",
        state.entity,
        state.current % data.states.len() + 1,
        data.states.len(),
        a.name,
    )
}
