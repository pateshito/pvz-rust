use crate::reanim::ReanimData;
use crate::resources::ViewerState;
use crate::Character;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub const BASE_ZOMBIE_TRACKS: &[&str] = &[
    "anim_head1",
    "anim_head2",
    "Zombie_body",
    "Zombie_neck",
    "Zombie_tie",
    "anim_innerarm1",
    "anim_innerarm2",
    "anim_innerarm3",
    "Zombie_outerarm_upper",
    "Zombie_outerarm_lower",
    "Zombie_outerarm_hand",
    "Zombie_innerleg_upper",
    "Zombie_innerleg_lower",
    "Zombie_innerleg_foot",
    "Zombie_outerleg_upper",
    "Zombie_outerleg_lower",
    "Zombie_outerleg_foot",
];

pub const BASE_PEASHOOTER_TRACKS: &[&str] = &[
    "anim_face",
    "idle_mouth",
    "idle_shoot_blink",
    "anim_blink",
    "PeaShooter_eyebrow",
    "backleaf",
    "backleaf_left_tip",
    "backleaf_right_tip",
    "frontleaf",
    "frontleaf_right_tip",
    "frontleaf_tip_left",
    "idle_headleaf_farthest",
    "idle_headleaf_3rdfarthest",
    "idle_headleaf_2ndfarthest",
    "idle_headleaf_nearest",
    "idle_headleaf_tip_top",
    "idle_headleaf_tip_bottom",
    "stalk_bottom",
    "stalk_top",
    "anim_stem",
    "anim_sprout",
];

pub fn get_allowlist(character: Character) -> &'static [&'static str] {
    match character {
        Character::Zombie => BASE_ZOMBIE_TRACKS,
        Character::PeaShooter => BASE_PEASHOOTER_TRACKS,
    }
}

pub fn image_to_path(internal: &str, character: Character) -> Option<String> {
    match character {
        Character::Zombie => {
            let rest = internal.strip_prefix("IMAGE_REANIM_ZOMBIE_")?;
            Some(format!(
                "textures/Zombie/Zombie_{}.png",
                rest.to_lowercase()
            ))
        }
        Character::PeaShooter => {
            // Manejar IMAGE_REANIM_ANIM_SPROUT (sin prefijo PEASHOOTER_)
            if let Some(rest) = internal.strip_prefix("IMAGE_REANIM_ANIM_") {
                return Some(format!(
                    "textures/PeaShooter/PeaShooter_{}.png",
                    rest.to_lowercase()
                ));
            }

            let rest = internal.strip_prefix("IMAGE_REANIM_PEASHOOTER_")?;
            let filename = match rest {
                "ANIM_SPROUT" => "PeaShooter_sprout.png",
                "HEADLEAF_FARTHEST" => "PeaShooter_headleaf_farthest.png",
                "HEADLEAF_3RDFARTHEST" => "PeaShooter_headleaf_3rdfarthest.png",
                // "2RD" es el nombre original de PopCap; el archivo también usa "2rd".
                "HEADLEAF_2RDFARTHEST" => "PeaShooter_headleaf_2rdfarthest.png",
                "HEADLEAF_NEAREST" => "PeaShooter_headleaf_nearest.png",
                "HEADLEAF_TIP_TOP" => "PeaShooter_headleaf_tip_top.png",
                "HEADLEAF_TIP_BOTTOM" => "PeaShooter_headleaf_tip_bottom.png",
                "FRONTLEAF_RIGHTTIP" => "PeaShooter_frontleaf_righttip.png",
                "FRONTLEAF_LEFTTIP" => "PeaShooter_frontleaf_lefttip.png",
                "BACKLEAF_LEFTTIP" => "PeaShooter_backleaf_lefttip.png",
                "BACKLEAF_RIGHTTIP" => "PeaShooter_backleaf_righttip.png",
                "BLINK1" => "PeaShooter_blink1.png",
                "BLINK2" => "PeaShooter_blink2.png",
                "EYEBROW" => "PeaShooter_eyebrow.png",
                "LIPS" => "PeaShooter_Lips.png",
                "MOUTH" => "PeaShooter_mouth.png",
                "HEAD" => "PeaShooter_Head.png",
                "STALK_TOP" => "PeaShooter_stalk_top.png",
                "STALK_BOTTOM" => "PeaShooter_stalk_bottom.png",
                "FRONTLEAF" => "PeaShooter_frontleaf.png",
                "BACKLEAF" => "PeaShooter_backleaf.png",
                other => {
                    return Some(format!(
                        "textures/PeaShooter/PeaShooter_{}.png",
                        other.to_lowercase()
                    ))
                }
            };
            Some(format!("textures/PeaShooter/{}", filename))
        }
    }
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
    if data.animations.is_empty() {
        return format!(
            "{:?} - sin animaciones\n←/→: N/A | Space | P | M | Esc",
            state.character
        );
    }
    let a = &data.animations[state.current % data.animations.len()];
    format!(
        "[{}/{}] {:?} - {}\n←/→: cambiar | Space: reiniciar | P: pausa{} | M: mover{} | Esc: menú",
        state.current % data.animations.len() + 1,
        data.animations.len(),
        state.character,
        a.name,
        if state.paused { " [PAUSADO]" } else { "" },
        if state.moving { " [MOVIMIENTO]" } else { "" },
    )
}
