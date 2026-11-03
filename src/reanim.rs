use regex::Regex;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Resolved {
    pub x: f32,
    pub y: f32,
    /// Ángulo (grados) del eje horizontal de la imagen.
    pub kx: f32,
    /// Ángulo (grados) del eje vertical de la imagen. Si difiere de `kx` la
    /// pieza queda sesgada (el cactus se inclina al disparar así).
    pub ky: f32,
    pub sx: f32,
    pub sy: f32,
    pub image: Option<String>,
    pub active: bool,
}

impl Resolved {
    /// Ejes de la pieza en el espacio de Bevy (y hacia arriba), que es el de un
    /// `Sprite` con ancla arriba a la izquierda. Devuelve `(eje_x, eje_y)`: a
    /// dónde va un paso hacia la derecha y un paso hacia arriba de la imagen.
    ///
    /// Con `kx == ky` es la rotación de siempre (`Quat::from_rotation_z(-kx)`);
    /// con `kx != ky` es un sesgo, que un `Transform` no puede representar.
    pub fn axes(&self) -> ([f32; 2], [f32; 2]) {
        let (kx, ky) = (self.kx.to_radians(), self.ky.to_radians());
        (
            [self.sx * kx.cos(), -self.sx * kx.sin()],
            [self.sy * ky.sin(), self.sy * ky.cos()],
        )
    }
}

/// A qué parte del personaje pertenece un track. Un estado puede leer cada
/// parte desde un rango de frames distinto (ver `AnimState`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PartGroup {
    #[default]
    Body,
    /// Cabeza principal (la de delante en el bisojo, la primera del triple).
    Head,
    /// Segunda cabeza: la de atrás del bisojo, o la del medio del triple.
    Head2,
    /// Tercera cabeza del triple.
    Head3,
}

#[derive(Clone, Debug)]
pub struct Track {
    pub name: String,
    pub frames: Vec<Option<Resolved>>,
    pub raw_f: Vec<Option<i32>>,
    pub has_image: bool,
    pub group: PartGroup,
}

#[derive(Clone, Debug)]
pub struct AnimRange {
    pub name: String,
    pub start: usize,
    pub end: usize,
}

/// Origen de frames de una cabeza extra (`Head2`, `Head3`).
#[derive(Clone, Debug)]
pub struct GroupSample {
    pub group: PartGroup,
    pub start: usize,
    /// Si está puesto y es menor que el largo del estado, esta parte se repite.
    pub cycle: Option<usize>,
    /// Balanceo horizontal por frame del cuerpo. Vacío = sin desplazamiento.
    pub dx: Vec<f32>,
}

/// Un estado que se puede reproducir (lo que el visor y la partida seleccionan).
/// Puede ser un rango del `.reanim` tal cual, o una combinación: el cuerpo y
/// cada cabeza leen frames de sitios distintos.
#[derive(Clone, Debug)]
pub struct AnimState {
    pub name: String,
    /// Cantidad de frames del bucle.
    pub len: usize,
    /// Primer frame de origen para los tracks del cuerpo.
    pub body_start: usize,
    /// Si está puesto, el cuerpo se repite cada tantos frames (el disparo de la
    /// ametralladora dura más que el balanceo del tallo).
    pub body_cycle: Option<usize>,
    /// Primer frame de origen para la cabeza principal.
    pub head_start: usize,
    /// Igual que `body_cycle`, para la cabeza principal.
    pub head_cycle: Option<usize>,
    /// Desplazamiento horizontal extra de la cabeza principal, indexado por el
    /// frame del cuerpo. Vacío = sin desplazamiento.
    pub head_dx: Vec<f32>,
    /// Cabezas que no son la principal.
    pub groups: Vec<GroupSample>,
}

impl AnimState {
    /// Cuerpo y cabeza leen el mismo rango. Es el estado de un `.reanim` sin combinar.
    pub fn same(name: impl Into<String>, start: usize, len: usize) -> Self {
        Self {
            name: name.into(),
            len,
            body_start: start,
            head_start: start,
            head_dx: Vec::new(),
            body_cycle: None,
            head_cycle: None,
            groups: Vec::new(),
        }
    }

    /// Frame de origen y desplazamiento horizontal de `group` en el frame `local`
    /// del bucle (0..len).
    pub fn sample(&self, group: PartGroup, local: usize) -> (usize, f32) {
        let body_step = step_of(local, self.body_cycle);
        match group {
            PartGroup::Body => (self.body_start + body_step, 0.0),
            PartGroup::Head => {
                let step = step_of(local, self.head_cycle);
                let dx = self.head_dx.get(body_step).copied().unwrap_or(0.0);
                (self.head_start + step, dx)
            }
            other => {
                if let Some(group) = self.groups.iter().find(|group| group.group == other) {
                    let step = step_of(local, group.cycle);
                    let dx = group.dx.get(body_step).copied().unwrap_or(0.0);
                    (group.start + step, dx)
                } else {
                    (self.body_start + body_step, 0.0)
                }
            }
        }
    }
}

fn step_of(local: usize, cycle: Option<usize>) -> usize {
    match cycle {
        Some(n) if n > 0 => local % n,
        _ => local,
    }
}

#[derive(Clone)]
pub struct ReanimData {
    pub fps: f32,
    pub tracks: Vec<Track>,
    pub animations: Vec<AnimRange>,
    pub states: Vec<AnimState>,
}

pub fn parse_reanim(path: &Path) -> Result<ReanimData, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("❌ No se pudo leer {:?}: {}", path, e))?;

    let fps = Regex::new(r"<fps>(\d+)</fps>")
        .unwrap()
        .captures(&content)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().parse::<f32>().ok())
        .unwrap_or(12.0);

    let track_re = Regex::new(r"(?s)<track>(.*?)</track>").unwrap();
    let name_re = Regex::new(r"(?s)<name>(.*?)</name>").unwrap();
    let frame_re = Regex::new(r"(?s)<t>(.*?)</t>").unwrap();
    let val_x = Regex::new(r"<x>(-?[\d.]+)</x>").unwrap();
    let val_y = Regex::new(r"<y>(-?[\d.]+)</y>").unwrap();
    let val_kx = Regex::new(r"<kx>(-?[\d.]+)</kx>").unwrap();
    let val_ky = Regex::new(r"<ky>(-?[\d.]+)</ky>").unwrap();
    let val_sx = Regex::new(r"<sx>(-?[\d.]+)</sx>").unwrap();
    let val_sy = Regex::new(r"<sy>(-?[\d.]+)</sy>").unwrap();
    let val_f = Regex::new(r"<f>(-?\d+)</f>").unwrap();
    let val_i = Regex::new(r"<i>(.*?)</i>").unwrap();

    let mut tracks: Vec<Track> = Vec::new();

    for track_cap in track_re.captures_iter(&content) {
        let tc = track_cap.get(1).unwrap().as_str();
        let name = name_re
            .captures(tc)
            .map(|c| c.get(1).unwrap().as_str().trim().to_string())
            .unwrap_or_else(|| "unknown".into());

        let mut frames: Vec<Option<Resolved>> = Vec::new();
        let mut raw_f: Vec<Option<i32>> = Vec::new();
        let mut has_image = false;

        let mut cur = Resolved {
            x: 0.0,
            y: 0.0,
            kx: 0.0,
            ky: 0.0,
            sx: 1.0,
            sy: 1.0,
            image: None,
            active: true,
        };

        for frame_cap in frame_re.captures_iter(tc) {
            let fc = frame_cap.get(1).unwrap().as_str();
            let get_f = |re: &Regex| -> Option<f32> {
                re.captures(fc)
                    .and_then(|c| c.get(1))
                    .and_then(|m| m.as_str().parse().ok())
            };

            if let Some(v) = get_f(&val_x) {
                cur.x = v;
            }
            if let Some(v) = get_f(&val_y) {
                cur.y = v;
            }
            if let Some(v) = get_f(&val_kx) {
                cur.kx = v;
            }
            if let Some(v) = get_f(&val_ky) {
                cur.ky = v;
            }
            if let Some(v) = get_f(&val_sx) {
                cur.sx = v;
            }
            if let Some(v) = get_f(&val_sy) {
                cur.sy = v;
            }

            if let Some(v) = val_i
                .captures(fc)
                .map(|c| c.get(1).unwrap().as_str().trim().to_string())
            {
                // `<i></i>` significa "sin imagen" (los créditos lo usan en
                // tracks que solo llevan `<text>`); no cuenta como sprite.
                cur.image = if v.is_empty() { None } else { Some(v.clone()) };
                if !v.is_empty() {
                    has_image = true;
                }
            }

            let f = val_f
                .captures(fc)
                .and_then(|c| c.get(1))
                .and_then(|m| m.as_str().parse::<i32>().ok());
            if let Some(v) = f {
                cur.active = v >= 0;
            }

            raw_f.push(f);
            frames.push(Some(cur.clone()));
        }

        tracks.push(Track {
            name,
            frames,
            raw_f,
            has_image,
            group: PartGroup::Body,
        });
    }

    let marker_range = |t: &Track| -> Option<(usize, usize)> {
        let mut active = true;
        let mut start: Option<usize> = None;
        let mut end = 0;
        for (i, f) in t.raw_f.iter().enumerate() {
            if let Some(v) = f {
                active = *v >= 0;
            }
            if active {
                if start.is_none() {
                    start = Some(i);
                }
                end = i;
            }
        }
        start.map(|s| (s, end))
    };

    let mut animations: Vec<AnimRange> = Vec::new();
    for t in &tracks {
        if t.has_image || !t.name.starts_with("anim_") {
            continue;
        }
        if let Some((s, e)) = marker_range(t) {
            animations.push(AnimRange {
                name: t.name.clone(),
                start: s,
                end: e,
            });
        }
    }
    animations.sort_by_key(|a| a.start);

    println!(
        "✅ {:?}: {} tracks | fps={} | {} animaciones",
        path.file_name(),
        tracks.len(),
        fps,
        animations.len()
    );
    for (i, a) in animations.iter().enumerate() {
        println!("   🎬 [{}] {} -> frames {}..{}", i, a.name, a.start, a.end);
    }

    // Por defecto, un estado por cada rango marcado en el `.reanim`, con cuerpo y
    // cabeza leyendo del mismo sitio. `plant_anim` sustituye esa lista en las
    // plantas cuyo archivo parte el cuerpo y las cabezas.
    //
    // Soles, proyectiles y varios props no traen tracks `anim_*`. Sin un estado
    // el visor no tendría nada que reproducir, así que se recorre el archivo entero.
    let states: Vec<AnimState> = if animations.is_empty() {
        let len = tracks.iter().map(|t| t.frames.len()).max().unwrap_or(0);
        if len == 0 {
            Vec::new()
        } else {
            vec![AnimState::same("full", 0, len)]
        }
    } else {
        animations
            .iter()
            .map(|a| AnimState::same(a.name.clone(), a.start, a.end.saturating_sub(a.start) + 1))
            .collect()
    };

    Ok(ReanimData {
        fps,
        tracks,
        animations,
        states,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(rel: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join(rel)
    }

    #[test]
    fn sin_marcadores_se_reproduce_el_archivo_entero() {
        let data = parse_reanim(&asset("animations/Sun/Sun.reanim")).expect("Sun.reanim");
        let len = data
            .tracks
            .iter()
            .map(|t| t.frames.len())
            .max()
            .unwrap_or(0);
        assert!(len > 1, "el sol tiene varios frames");
        assert_eq!(data.animations.len(), 0);
        assert_eq!(data.states.len(), 1);
        assert_eq!(data.states[0].name, "full");
        assert_eq!(data.states[0].len, len);
        assert_eq!(data.states[0].body_start, 0);
    }

    #[test]
    fn los_marcadores_anim_siguen_siendo_los_estados() {
        let data =
            parse_reanim(&asset("animations/PeaShooter/PeaShooter.reanim")).expect("PeaShooter");
        assert!(data.states.iter().any(|s| s.name == "anim_idle"));
        assert!(data.states.iter().all(|s| s.name != "full"));
    }

    #[test]
    fn sin_sesgo_los_ejes_son_la_rotacion_de_siempre() {
        let r = Resolved {
            x: 0.0,
            y: 0.0,
            kx: 30.0,
            ky: 30.0,
            sx: 2.0,
            sy: 0.5,
            image: None,
            active: true,
        };
        let (ex, ey) = r.axes();
        // Es lo que hacía `Quat::from_rotation_z(-kx)` con la escala (sx, sy).
        let a = (-30.0_f32).to_radians();
        let close = |u: f32, v: f32| (u - v).abs() < 1e-5;
        assert!(close(ex[0], 2.0 * a.cos()) && close(ex[1], 2.0 * a.sin()));
        assert!(close(ey[0], -0.5 * a.sin()) && close(ey[1], 0.5 * a.cos()));
    }

    /// X (espacio Bevy) de la esquina inferior izquierda de la pieza en un frame.
    fn cactus_base_x(data: &ReanimData, track: &str, frame: usize) -> f32 {
        let t = data.tracks.iter().find(|t| t.name == track).expect(track);
        let r = t.frames[frame].as_ref().unwrap();
        let name = r
            .image
            .as_ref()
            .unwrap()
            .trim_start_matches("IMAGE_REANIM_CACTUS_")
            .to_lowercase();
        let png = std::fs::read(asset(&format!("textures/Cactus/Cactus_{name}.png"))).unwrap();
        let h = u32::from_be_bytes(png[20..24].try_into().unwrap()) as f32;
        r.x - h * r.axes().1[0]
    }

    /// Al disparar, el cactus se inclina hacia delante (sesgo `ky`) pero su base
    /// sigue en el suelo. Sin el sesgo el cuerpo queda recto y se parte de la
    /// cabeza: la base se movía ~22 px.
    #[test]
    fn el_cactus_se_inclina_sin_despegar_la_base() {
        let data = parse_reanim(&asset("animations/Cactus/Cactus.reanim")).expect("Cactus");
        let frames = |name: &str| {
            let s = data.states.iter().find(|s| s.name == name).expect(name);
            s.body_start..s.body_start + s.len
        };
        let check = |track: &str, anim: &str, idle: &str, max: f32| {
            let reference = cactus_base_x(&data, track, frames(idle).start);
            for f in frames(anim) {
                let d = (cactus_base_x(&data, track, f) - reference).abs();
                assert!(
                    d <= max,
                    "{track} en {anim}, frame {f}: la base se mueve {d:.1} px (máx {max})"
                );
            }
        };
        check("anim_face", "anim_shooting", "anim_idle", 3.0);
        check("Cactus_body3", "anim_shootinghigh", "anim_idlehigh", 5.0);
        check("body_overlay", "anim_shootinghigh", "anim_idlehigh", 1.0);
    }
}
