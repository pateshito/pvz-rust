//! Estados del PeaShooter: solo `idle` y `shooting`, siempre con la planta completa.
//!
//! El `.reanim` original reparte la animación en 4 bloques de la misma duración:
//!
//! | marcador         | contenido                              |
//! |------------------|----------------------------------------|
//! | `anim_idle`      | solo el tallo y las hojas de abajo     |
//! | `anim_head_idle` | solo la cabeza, en reposo              |
//! | `anim_shooting`  | solo la cabeza, disparando             |
//! | `anim_full_idle` | tallo y cabeza juntos, en reposo       |
//!
//! Aquí se unen en dos estados que se pueden cambiar en cualquier momento:
//!
//! - `idle`: `anim_full_idle` tal cual (ya es tallo + cabeza).
//! - `shooting`: tallo de `anim_idle` + cabeza de `anim_shooting`. Para que la
//!   cabeza siga al tallo como en `idle`, se le suma el balanceo horizontal del
//!   track `anim_stem` (así está hecho `anim_full_idle` respecto a
//!   `anim_head_idle`, con ~0,5 px de diferencia).
//!
//! Como el tallo avanza igual en ambos estados, el cambio entre ellos no da saltos.

use crate::reanim::{AnimState, PartGroup, ReanimData, Track};

pub const STATE_IDLE: &str = "idle";
pub const STATE_SHOOTING: &str = "shooting";

/// Reemplaza los estados de `data` por `idle` y `shooting`, y marca qué tracks
/// son de la cabeza. Devuelve un error si el `.reanim` no tiene la estructura esperada.
pub fn build_states(data: &mut ReanimData) -> Result<(), String> {
    let range = |name: &str| -> Result<(usize, usize), String> {
        data.animations
            .iter()
            .find(|a| a.name == name)
            .map(|a| (a.start, a.end))
            .ok_or_else(|| format!("❌ PeaShooter: falta la animación '{}'", name))
    };

    let (stalk_start, stalk_end) = range("anim_idle")?;
    let (head_idle_start, _) = range("anim_head_idle")?;
    let (shoot_start, shoot_end) = range("anim_shooting")?;
    let (full_start, full_end) = range("anim_full_idle")?;

    let len = full_end - full_start + 1;
    if stalk_end - stalk_start + 1 != len || shoot_end - shoot_start + 1 != len {
        return Err(format!(
            "❌ PeaShooter: anim_idle, anim_shooting y anim_full_idle deben durar lo mismo ({} frames esperados)",
            len
        ));
    }

    // Balanceo horizontal del tallo respecto a su primer frame.
    let stem = data
        .tracks
        .iter()
        .find(|t| t.name == "anim_stem")
        .ok_or("❌ PeaShooter: falta el track 'anim_stem'")?;
    let stem_x = |frame: usize| -> f32 {
        stem.frames
            .get(frame)
            .and_then(|r| r.as_ref())
            .map_or(0.0, |r| r.x)
    };
    let base_x = stem_x(stalk_start);
    let head_dx: Vec<f32> = (0..len).map(|k| stem_x(stalk_start + k) - base_x).collect();

    // Una pieza es de la cabeza si aparece en algún frame de los bloques de la cabeza
    // (reposo o disparo; ej. el parpadeo de disparar solo existe en `anim_shooting`)
    // y en ningún frame del bloque del tallo.
    let active_in = |t: &Track, from: usize, to: usize| {
        (from..=to).any(|f| {
            t.frames
                .get(f)
                .and_then(|r| r.as_ref())
                .map_or(false, |r| r.active)
        })
    };
    for track in &mut data.tracks {
        track.group = if active_in(track, head_idle_start, shoot_end)
            && !active_in(track, stalk_start, stalk_end)
        {
            PartGroup::Head
        } else {
            PartGroup::Body
        };
    }

    data.states = vec![
        AnimState {
            name: STATE_IDLE.into(),
            len,
            body_start: full_start,
            head_start: full_start,
            head_dx: Vec::new(),
        },
        AnimState {
            name: STATE_SHOOTING.into(),
            len,
            body_start: stalk_start,
            head_start: shoot_start,
            head_dx,
        },
    ];

    println!("🌱 PeaShooter: estados '{}' y '{}' ({} frames)", STATE_IDLE, STATE_SHOOTING, len);
    Ok(())
}

/// Índice de un estado por nombre; pensado para que la partida cambie entre
/// `idle` y `shooting` sin depender del orden de la lista.
#[allow(dead_code)]
pub fn state_index(data: &ReanimData, name: &str) -> Option<usize> {
    data.states.iter().position(|s| s.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reanim::parse_reanim;
    use std::path::Path;

    fn load() -> ReanimData {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/animations/PeaShooter/PeaShooter.reanim");
        let mut data = parse_reanim(&path).expect("no se pudo leer PeaShooter.reanim");
        build_states(&mut data).expect("build_states falló");
        data
    }

    fn track<'a>(data: &'a ReanimData, name: &str) -> &'a Track {
        data.tracks.iter().find(|t| t.name == name).unwrap()
    }

    #[test]
    fn solo_hay_idle_y_shooting() {
        let data = load();
        let names: Vec<&str> = data.states.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, [STATE_IDLE, STATE_SHOOTING]);
        assert_eq!(state_index(&data, STATE_IDLE), Some(0));
        assert_eq!(state_index(&data, STATE_SHOOTING), Some(1));
        assert!(data.states.iter().all(|s| s.len == 25));
    }

    #[test]
    fn cabeza_y_cuerpo_se_separan_bien() {
        let data = load();
        for head in [
            "anim_face",
            "idle_mouth",
            "idle_headleaf_farthest",
            "PeaShooter_eyebrow",
            "idle_shoot_blink",
        ] {
            assert_eq!(track(&data, head).group, PartGroup::Head, "{head}");
        }
        for body in ["stalk_top", "stalk_bottom", "backleaf", "frontleaf", "anim_stem"] {
            assert_eq!(track(&data, body).group, PartGroup::Body, "{body}");
        }
    }

    #[test]
    fn idle_usa_full_idle_y_shooting_combina_tallo_y_cabeza() {
        let data = load();
        let start = |n: &str| data.animations.iter().find(|a| a.name == n).unwrap().start;
        let (idle, shooting) = (&data.states[0], &data.states[1]);
        assert_eq!(
            (idle.body_start, idle.head_start),
            (start("anim_full_idle"), start("anim_full_idle"))
        );
        assert_eq!(
            (shooting.body_start, shooting.head_start),
            (start("anim_idle"), start("anim_shooting"))
        );
    }

    #[test]
    fn la_cabeza_sigue_el_balanceo_del_tallo() {
        let data = load();
        let dx = &data.states[1].head_dx;
        assert_eq!(dx.len(), 25);
        assert_eq!(dx[0], 0.0);
        assert!(
            dx.iter().cloned().fold(0.0_f32, f32::max) > 5.0,
            "el tallo se balancea"
        );
    }

    /// La regla usada para `shooting` (cabeza + balanceo del tallo) reproduce, en
    /// reposo, la cabeza de `anim_full_idle` con menos de ~1 px de error en x.
    #[test]
    fn la_regla_reproduce_full_idle_con_menos_de_1px() {
        let data = load();
        let start = |n: &str| data.animations.iter().find(|a| a.name == n).unwrap().start;
        let (head_idle, full) = (start("anim_head_idle"), start("anim_full_idle"));
        let dx = &data.states[1].head_dx;
        let mut comparados = 0;
        for t in data
            .tracks
            .iter()
            .filter(|t| t.group == PartGroup::Head && t.has_image)
        {
            for k in 0..25 {
                let (Some(Some(h)), Some(Some(f))) =
                    (t.frames.get(head_idle + k), t.frames.get(full + k))
                else {
                    continue;
                };
                if !h.active || !f.active {
                    continue;
                }
                let err = (h.x + dx[k] - f.x).abs();
                assert!(err < 1.01, "{} frame {k}: error {err}", t.name);
                comparados += 1;
            }
        }
        assert!(comparados > 100, "se compararon pocas piezas: {comparados}");
    }
}
