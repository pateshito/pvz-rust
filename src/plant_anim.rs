//! Une los `.reanim` que traen el cuerpo y las cabezas en rangos separados.
//!
//! El visor elige un estado y cada pieza lee el frame de su parte. Las plantas
//! que ya traen el personaje entero en cada `anim_*` se quedan como las dejó
//! el parser.
//!
//! - PeaShooter y SnowPea: `idle` (`anim_full_idle`) y `shooting` (tallo de
//!   `anim_idle` + cabeza de `anim_shooting`). El balanceo sale de `anim_stem`.
//! - GatlingPea: `idle` (tallo + cabeza en reposo) y `shooting`. El disparo
//!   dura 39 frames y el tallo, de 25, se repite. El balanceo sale de `stalk_top`.
//! - SplitPea: `idle`, `shooting` (dispara la de delante) y `back_shooting`
//!   (la de atrás dispara en un ciclo de 13; cuerpo y cabeza delantera siguen
//!   el reposo de 25).
//! - ThreePeater: `idle` y `shooting`. En el disparo las tres cabezas salen
//!   juntas (13 frames) y el tallo de 25 se repite.

use crate::reanim::{AnimState, GroupSample, PartGroup, ReanimData, Track};

const IDLE: &str = "idle";
const SHOOTING: &str = "shooting";
const BACK_SHOOTING: &str = "back_shooting";

/// Aplica la receta de la planta cuando el archivo es el principal de la entidad.
/// Una variante (por ejemplo `PeaShooterSingle`) se deja como viene.
pub fn apply(entity: &str, reanim: &str, data: &mut ReanimData) -> Result<(), String> {
    if entity != reanim {
        return Ok(());
    }
    match entity {
        "PeaShooter" | "SnowPea" => compose_standard(data, entity, &["anim_stem"]),
        "GatlingPea" => compose_standard(data, entity, &["anim_stem", "stalk_top"]),
        "SplitPea" => compose_split(data),
        "ThreePeater" => compose_triple(data),
        _ => Ok(()),
    }
}

/// `idle` + `shooting` para un tirador de una cabeza.
///
/// Si existe `anim_full_idle`, el reposo usa ese bloque (tallo y cabeza ya juntos)
/// y las duraciones tienen que coincidir. Si no existe, el reposo junta
/// `anim_idle` con `anim_head_idle`. El disparo siempre toma el tallo de
/// `anim_idle` y la cabeza de `anim_shooting`; si el disparo es más largo, el
/// tallo se repite.
///
/// `stem_names` es la lista de tracks de balanceo, en orden. PeaShooter pasa
/// solo `anim_stem`: usar `stalk_top` rompería el reposo completo, que sigue
/// al tallo con menos de 1 px.
pub fn compose_standard(
    data: &mut ReanimData,
    label: &str,
    stem_names: &[&str],
) -> Result<(), String> {
    let (body_s, body_e) = anim_range(data, label, "anim_idle")?;
    let (head_s, head_e) = anim_range(data, label, "anim_head_idle")?;
    let (shoot_s, shoot_e) = anim_range(data, label, "anim_shooting")?;
    let full = data
        .animations
        .iter()
        .find(|a| a.name == "anim_full_idle")
        .map(|a| (a.start, a.end));

    let body_len = span_len(body_s, body_e);
    let head_len = span_len(head_s, head_e);
    let shoot_len = span_len(shoot_s, shoot_e);

    if let Some((full_s, full_e)) = full {
        let full_len = span_len(full_s, full_e);
        if body_len != full_len || head_len != full_len || shoot_len != full_len {
            return Err(format!(
                "❌ {label}: anim_idle ({body_len}), anim_head_idle ({head_len}), anim_shooting ({shoot_len}) y anim_full_idle ({full_len}) deben durar lo mismo"
            ));
        }
    } else if head_len != body_len {
        return Err(format!(
            "❌ {label}: anim_idle ({body_len}) y anim_head_idle ({head_len}) deben durar lo mismo"
        ));
    }

    let dx = sway(find_stem(data, label, stem_names)?, body_s, body_len);
    let groups = classify(
        data,
        (body_s, body_e),
        &[(PartGroup::Head, head_s, shoot_e)],
    );
    let idle = if let Some((full_s, _)) = full {
        AnimState::same(IDLE, full_s, body_len)
    } else {
        plant_state(
            IDLE,
            body_len,
            body_s,
            None,
            head_s,
            None,
            dx.clone(),
            Vec::new(),
        )
    };
    let shooting = plant_state(
        SHOOTING,
        shoot_len,
        body_s,
        (shoot_len != body_len).then_some(body_len),
        shoot_s,
        None,
        dx,
        Vec::new(),
    );
    commit(data, groups, vec![idle, shooting]);
    println!(
        "🌱 {label}: estados '{IDLE}' ({body_len} frames) y '{SHOOTING}' ({shoot_len} frames)"
    );
    Ok(())
}

fn compose_split(data: &mut ReanimData) -> Result<(), String> {
    let label = "SplitPea";
    let (body_s, body_e) = anim_range(data, label, "anim_idle")?;
    let (front_idle_s, front_idle_e) = anim_range(data, label, "anim_head_idle")?;
    let (front_shoot_s, front_shoot_e) = anim_range(data, label, "anim_shooting")?;
    let (back_idle_s, back_idle_e) = anim_range(data, label, "anim_splitpea_idle")?;
    let (back_shoot_s, back_shoot_e) = anim_range(data, label, "anim_splitpea_shooting")?;

    let body_len = span_len(body_s, body_e);
    let front_idle_len = span_len(front_idle_s, front_idle_e);
    let front_shoot_len = span_len(front_shoot_s, front_shoot_e);
    let back_idle_len = span_len(back_idle_s, back_idle_e);
    let back_shoot_len = span_len(back_shoot_s, back_shoot_e);
    if body_len != front_idle_len || body_len != front_shoot_len || body_len != back_idle_len {
        return Err(format!(
            "❌ {label}: el cuerpo y los reposos de las dos cabezas deben durar lo mismo ({body_len}, {front_idle_len}, {front_shoot_len}, {back_idle_len})"
        ));
    }

    let dx = sway(
        find_stem(data, label, &["anim_stem", "stalk_top"])?,
        body_s,
        body_len,
    );
    let groups = classify(
        data,
        (body_s, body_e),
        &[
            (PartGroup::Head, front_idle_s, front_shoot_e),
            (PartGroup::Head2, back_idle_s, back_shoot_e),
        ],
    );

    let idle = plant_state(
        IDLE,
        body_len,
        body_s,
        None,
        front_idle_s,
        None,
        dx.clone(),
        vec![extra(PartGroup::Head2, back_idle_s, None, dx.clone())],
    );
    let shooting = plant_state(
        SHOOTING,
        body_len,
        body_s,
        None,
        front_shoot_s,
        None,
        dx.clone(),
        vec![extra(PartGroup::Head2, back_idle_s, None, dx.clone())],
    );
    // El disparo trasero dura menos que el balanceo. La cabeza de atrás se
    // repite y el cuerpo sigue el ciclo largo, así el tallo no se congela.
    let back = plant_state(
        BACK_SHOOTING,
        body_len,
        body_s,
        None,
        front_idle_s,
        None,
        dx.clone(),
        vec![extra(
            PartGroup::Head2,
            back_shoot_s,
            Some(back_shoot_len),
            dx,
        )],
    );
    commit(data, groups, vec![idle, shooting, back]);
    println!(
        "🌱 {label}: estados '{IDLE}', '{SHOOTING}' y '{BACK_SHOOTING}' ({body_len} frames, disparo trasero {back_shoot_len})"
    );
    Ok(())
}

fn compose_triple(data: &mut ReanimData) -> Result<(), String> {
    let label = "ThreePeater";
    let body = anim_range(data, label, "anim_head1")?;
    let head2_marker = anim_range(data, label, "anim_head2")?;
    let head3_marker = anim_range(data, label, "anim_head3")?;
    if head2_marker != body || head3_marker != body {
        return Err(format!(
            "❌ {label}: anim_head1, anim_head2 y anim_head3 deben marcar el mismo bloque del tallo"
        ));
    }
    let (body_s, body_e) = body;
    let (idle1_s, idle1_e) = anim_range(data, label, "anim_head_idle1")?;
    let (shoot1_s, shoot1_e) = anim_range(data, label, "anim_shooting1")?;
    let (idle2_s, idle2_e) = anim_range(data, label, "anim_head_idle2")?;
    let (shoot2_s, shoot2_e) = anim_range(data, label, "anim_shooting2")?;
    let (idle3_s, idle3_e) = anim_range(data, label, "anim_head_idle3")?;
    let (shoot3_s, shoot3_e) = anim_range(data, label, "anim_shooting3")?;

    let body_len = span_len(body_s, body_e);
    let idle_lens = [
        span_len(idle1_s, idle1_e),
        span_len(idle2_s, idle2_e),
        span_len(idle3_s, idle3_e),
    ];
    let shoot_lens = [
        span_len(shoot1_s, shoot1_e),
        span_len(shoot2_s, shoot2_e),
        span_len(shoot3_s, shoot3_e),
    ];
    if idle_lens.iter().any(|len| *len != body_len) {
        return Err(format!(
            "❌ {label}: cada cabeza en reposo debe durar lo mismo que el tallo ({body_len})"
        ));
    }
    let shoot_len = shoot_lens[0];
    if shoot_lens.iter().any(|len| *len != shoot_len) || shoot_len == 0 {
        return Err(format!(
            "❌ {label}: los tres disparos deben durar lo mismo ({shoot_lens:?})"
        ));
    }

    let dx = sway(
        find_stem(data, label, &["ThreePeater_stem2", "ThreePeater_stem1"])?,
        body_s,
        body_len,
    );
    let groups = classify(
        data,
        (body_s, body_e),
        &[
            (PartGroup::Head, idle1_s, shoot1_e),
            (PartGroup::Head2, idle2_s, shoot2_e),
            (PartGroup::Head3, idle3_s, shoot3_e),
        ],
    );
    let idle = plant_state(
        IDLE,
        body_len,
        body_s,
        None,
        idle1_s,
        None,
        dx.clone(),
        vec![
            extra(PartGroup::Head2, idle2_s, None, dx.clone()),
            extra(PartGroup::Head3, idle3_s, None, dx.clone()),
        ],
    );
    let shooting = plant_state(
        SHOOTING,
        shoot_len,
        body_s,
        Some(body_len),
        shoot1_s,
        None,
        dx.clone(),
        vec![
            extra(PartGroup::Head2, shoot2_s, None, dx.clone()),
            extra(PartGroup::Head3, shoot3_s, None, dx),
        ],
    );
    commit(data, groups, vec![idle, shooting]);
    println!(
        "🌱 {label}: estados '{IDLE}' ({body_len} frames) y '{SHOOTING}' ({shoot_len} frames, tres cabezas)"
    );
    Ok(())
}

fn anim_range(data: &ReanimData, label: &str, name: &str) -> Result<(usize, usize), String> {
    data.animations
        .iter()
        .find(|a| a.name == name)
        .map(|a| (a.start, a.end))
        .ok_or_else(|| format!("❌ {label}: falta la animación '{name}'"))
}

fn span_len(start: usize, end: usize) -> usize {
    end - start + 1
}

fn find_stem<'a>(data: &'a ReanimData, label: &str, names: &[&str]) -> Result<&'a Track, String> {
    names
        .iter()
        .find_map(|name| data.tracks.iter().find(|t| t.name == *name))
        .ok_or_else(|| format!("❌ {label}: falta el track de tallo ({})", names.join(", ")))
}

/// Desplazamiento horizontal respecto al primer frame del cuerpo.
fn sway(track: &Track, start: usize, len: usize) -> Vec<f32> {
    let base = frame_x(track, start);
    (0..len).map(|k| frame_x(track, start + k) - base).collect()
}

fn frame_x(track: &Track, frame: usize) -> f32 {
    track
        .frames
        .get(frame)
        .and_then(|r| r.as_ref())
        .map_or(0.0, |r| r.x)
}

fn active_in(track: &Track, from: usize, to: usize) -> bool {
    (from..=to).any(|f| {
        track
            .frames
            .get(f)
            .and_then(|r| r.as_ref())
            .map_or(false, |r| r.active)
    })
}

/// Una pieza es de una cabeza si aparece en el bloque de esa cabeza y en
/// ningún frame del cuerpo. El orden de `parts` desempata si dos cabezas coincidieran.
fn classify(
    data: &ReanimData,
    body: (usize, usize),
    parts: &[(PartGroup, usize, usize)],
) -> Vec<PartGroup> {
    data.tracks
        .iter()
        .map(|track| {
            if active_in(track, body.0, body.1) {
                return PartGroup::Body;
            }
            parts
                .iter()
                .find(|(_, from, to)| active_in(track, *from, *to))
                .map(|(group, _, _)| *group)
                .unwrap_or(PartGroup::Body)
        })
        .collect()
}

fn extra(group: PartGroup, start: usize, cycle: Option<usize>, dx: Vec<f32>) -> GroupSample {
    GroupSample {
        group,
        start,
        cycle,
        dx,
    }
}

fn plant_state(
    name: &str,
    len: usize,
    body_start: usize,
    body_cycle: Option<usize>,
    head_start: usize,
    head_cycle: Option<usize>,
    head_dx: Vec<f32>,
    groups: Vec<GroupSample>,
) -> AnimState {
    AnimState {
        name: name.into(),
        len,
        body_start,
        body_cycle,
        head_start,
        head_cycle,
        head_dx,
        groups,
    }
}

fn commit(data: &mut ReanimData, groups: Vec<PartGroup>, states: Vec<AnimState>) {
    for (track, group) in data.tracks.iter_mut().zip(groups) {
        track.group = group;
    }
    data.states = states;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reanim::parse_reanim;
    use std::path::Path;

    fn load(rel: &str) -> ReanimData {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join(rel);
        parse_reanim(&path).unwrap_or_else(|e| panic!("{rel}: {e}"))
    }

    fn composed(entity: &str, rel: &str) -> ReanimData {
        let mut data = load(rel);
        apply(entity, entity, &mut data).unwrap_or_else(|e| panic!("{entity}: {e}"));
        data
    }

    fn start(data: &ReanimData, name: &str) -> usize {
        data.animations
            .iter()
            .find(|a| a.name == name)
            .unwrap()
            .start
    }

    fn track<'a>(data: &'a ReanimData, name: &str) -> &'a Track {
        data.tracks.iter().find(|t| t.name == name).expect(name)
    }

    fn names(data: &ReanimData) -> Vec<&str> {
        data.states.iter().map(|s| s.name.as_str()).collect()
    }

    #[test]
    fn snowpea_queda_en_idle_y_shooting() {
        let data = composed("SnowPea", "animations/SnowPea/SnowPea.reanim");
        assert_eq!(names(&data), ["idle", "shooting"]);
        assert!(data.states.iter().all(|s| s.len == 25));
        let (idle, shooting) = (&data.states[0], &data.states[1]);
        assert_eq!(idle.body_start, start(&data, "anim_full_idle"));
        assert_eq!(idle.head_start, start(&data, "anim_full_idle"));
        assert!(idle.head_dx.is_empty());
        assert_eq!(shooting.body_start, start(&data, "anim_idle"));
        assert_eq!(shooting.head_start, start(&data, "anim_shooting"));
        assert_eq!(track(&data, "anim_face").group, PartGroup::Head);
        assert_eq!(track(&data, "SnowPea_crystals1").group, PartGroup::Head);
        assert_eq!(track(&data, "stalk_top").group, PartGroup::Body);
        assert_eq!(track(&data, "anim_stem").group, PartGroup::Body);
        let peak = shooting.head_dx.iter().copied().fold(0.0_f32, f32::max);
        assert!(peak > 5.0, "el tallo se balancea");
    }

    #[test]
    fn gatling_repite_el_tallo_durante_el_disparo_largo() {
        let data = composed("GatlingPea", "animations/GatlingPea/GatlingPea.reanim");
        assert_eq!(names(&data), ["idle", "shooting"]);
        let (idle, shooting) = (&data.states[0], &data.states[1]);
        assert_eq!(idle.len, 25);
        assert_eq!(idle.body_cycle, None);
        assert_eq!(idle.body_start, start(&data, "anim_idle"));
        assert_eq!(idle.head_start, start(&data, "anim_head_idle"));
        assert_eq!(shooting.len, 39);
        assert_eq!(shooting.body_cycle, Some(25));
        assert_eq!(shooting.head_start, start(&data, "anim_shooting"));
        assert_eq!(track(&data, "GatlingPea_barrel1").group, PartGroup::Head);
        assert_eq!(track(&data, "stalk_top").group, PartGroup::Body);
        assert_eq!(
            shooting.sample(PartGroup::Body, 30).0,
            shooting.body_start + 5
        );
        assert_eq!(
            shooting.sample(PartGroup::Head, 30).0,
            shooting.head_start + 30
        );
    }

    #[test]
    fn splitpea_tiene_tres_estados_y_dos_cabezas() {
        let data = composed("SplitPea", "animations/SplitPea/SplitPea.reanim");
        assert_eq!(names(&data), ["idle", "shooting", "back_shooting"]);
        assert!(data.states.iter().all(|s| s.len == 25));
        assert_eq!(track(&data, "anim_face").group, PartGroup::Head);
        assert_eq!(track(&data, "idle_mouth").group, PartGroup::Head);
        assert_eq!(track(&data, "SplitPea_head").group, PartGroup::Head2);
        assert_eq!(track(&data, "SplitPea_mouth").group, PartGroup::Head2);
        assert_eq!(track(&data, "stalk_top").group, PartGroup::Body);

        let shooting = &data.states[1];
        assert_eq!(
            shooting.sample(PartGroup::Head, 0).0,
            start(&data, "anim_shooting")
        );
        assert_eq!(
            shooting.sample(PartGroup::Head2, 0).0,
            start(&data, "anim_splitpea_idle")
        );

        let back = &data.states[2];
        let back_start = start(&data, "anim_splitpea_shooting");
        assert_eq!(back.sample(PartGroup::Head2, 0).0, back_start);
        assert_eq!(back.sample(PartGroup::Head2, 13).0, back_start);
        assert_eq!(back.sample(PartGroup::Head2, 14).0, back_start + 1);
        assert_eq!(
            back.sample(PartGroup::Head, 13).0,
            start(&data, "anim_head_idle") + 13
        );
        assert_eq!(back.sample(PartGroup::Body, 13).0, back.body_start + 13);
    }

    #[test]
    fn threepeater_dispara_las_tres_cabezas_juntas() {
        let data = composed("ThreePeater", "animations/ThreePeater/ThreePeater.reanim");
        assert_eq!(names(&data), ["idle", "shooting"]);
        let (idle, shooting) = (&data.states[0], &data.states[1]);
        assert_eq!(idle.len, 25);
        assert_eq!(shooting.len, 13);
        assert_eq!(shooting.body_cycle, Some(25));
        assert_eq!(track(&data, "anim_face1").group, PartGroup::Head);
        assert_eq!(track(&data, "ThreePeater_mouth1").group, PartGroup::Head);
        assert_eq!(track(&data, "anim_face2").group, PartGroup::Head2);
        assert_eq!(track(&data, "ThreePeater_mouth2").group, PartGroup::Head2);
        assert_eq!(track(&data, "anim_face3").group, PartGroup::Head3);
        assert_eq!(track(&data, "ThreePeater_mouth3").group, PartGroup::Head3);
        assert_eq!(track(&data, "ThreePeater_stem2").group, PartGroup::Body);
        assert_eq!(
            idle.sample(PartGroup::Head2, 0).0,
            start(&data, "anim_head_idle2")
        );
        assert_eq!(
            shooting.sample(PartGroup::Body, 0).0,
            start(&data, "anim_head1")
        );
        assert_eq!(
            shooting.sample(PartGroup::Head, 0).0,
            start(&data, "anim_shooting1")
        );
        assert_eq!(
            shooting.sample(PartGroup::Head3, 0).0,
            start(&data, "anim_shooting3")
        );
        let peak = shooting
            .head_dx
            .iter()
            .copied()
            .map(f32::abs)
            .fold(0.0_f32, f32::max);
        assert!(peak > 5.0, "el tallo se balancea");
    }

    #[test]
    fn el_zombie_conserva_sus_animaciones() {
        let mut data = load("animations/Zombie/Zombie.reanim");
        let before: Vec<String> = data.states.iter().map(|s| s.name.clone()).collect();
        apply("Zombie", "Zombie", &mut data).unwrap();
        let after: Vec<String> = data.states.iter().map(|s| s.name.clone()).collect();
        assert_eq!(before, after);
        for name in ["anim_walk", "anim_eat", "anim_death"] {
            assert!(after.iter().any(|s| s == name), "{name}");
        }
        assert!(after.iter().all(|s| s != "idle"));
    }

    #[test]
    fn la_nuez_y_la_variante_del_lanzaguisantes_no_se_reescriben() {
        let mut wall = load("animations/Wallnut/Wallnut.reanim");
        let before: Vec<String> = wall.states.iter().map(|s| s.name.clone()).collect();
        apply("Wallnut", "Wallnut", &mut wall).unwrap();
        let after: Vec<String> = wall.states.iter().map(|s| s.name.clone()).collect();
        assert_eq!(before, after);

        let mut single = load("animations/PeaShooter/PeaShooterSingle.reanim");
        apply("PeaShooter", "PeaShooterSingle", &mut single).unwrap();
        assert!(single.states.iter().any(|s| s.name == "anim_full_idle"));
        assert!(single.states.iter().all(|s| s.name != "idle"));
    }

    #[test]
    fn un_error_deja_el_reanim_como_estaba() {
        let mut data = load("animations/PeaShooter/PeaShooter.reanim");
        let before: Vec<String> = data.states.iter().map(|s| s.name.clone()).collect();
        let err = compose_standard(&mut data, "PeaShooter", &["no_existe"]).unwrap_err();
        assert!(err.contains("no_existe"), "{err}");
        let after: Vec<String> = data.states.iter().map(|s| s.name.clone()).collect();
        assert_eq!(before, after);
        assert!(data.tracks.iter().all(|t| t.group == PartGroup::Body));
    }
}
