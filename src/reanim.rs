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

/// Un parpadeo dibujado encima de un estado. El cuerpo sigue en el frame del
/// reposo; estos tracks se leen del clip y tapan los ojos abiertos.
#[derive(Clone, Debug)]
pub struct BlinkClip {
    /// Primer frame de origen del clip, inclusive.
    pub start: usize,
    /// Largo en frames, hueco de ojos abiertos incluido.
    pub len: usize,
    /// Tracks del párpado. Se muestrean en `start + frame`.
    pub tracks: Vec<usize>,
    /// Ojos sueltos que se ocultan mientras el párpado de este clip está visible.
    pub hide: Vec<usize>,
}

/// Segundos de reposo entre un parpadeo y el siguiente.
pub const BLINK_PERIOD: f32 = 4.5;

/// Un estado que se puede reproducir (lo que el visor y la partida seleccionan).
/// Puede ser un rango del `.reanim` tal cual, o una combinación: el cuerpo y
/// cada cabeza leen frames de sitios distintos.
///
/// `blinks` no es otro estado: se dibuja de vez en cuando encima de este.
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
    /// Parpadeos que se superponen a este estado, de vez en cuando.
    pub blinks: Vec<BlinkClip>,
}

/// Qué hacer con un track mientras hay un parpadeo en curso.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overlay {
    /// Leer este frame de origen (el del párpado, no el del reposo).
    Sample(usize),
    /// Tapar la pieza: el párpado ocupa su lugar.
    Hidden,
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
            blinks: Vec::new(),
        }
    }

    /// Clip y frame dentro del clip si en `time` (segundos) toca parpadear.
    /// `phase` desfasa copias del mismo estado para que no parpadeen juntas.
    /// El parpadeo cae al final de cada periodo, así al aparecer se ven los ojos abiertos.
    pub fn blink_at(&self, time: f32, fps: f32, phase: f32) -> Option<(usize, usize)> {
        if self.blinks.is_empty() || fps <= 0.0 {
            return None;
        }
        let time = time + phase.rem_euclid(BLINK_PERIOD);
        let n = (time / BLINK_PERIOD) as usize;
        let clip_i = n % self.blinks.len();
        let into = time - n as f32 * BLINK_PERIOD;
        let dur = self.blinks[clip_i].len as f32 / fps;
        let begin = (BLINK_PERIOD - dur).max(0.0);
        if into < begin {
            return None;
        }
        let frame = ((into - begin) * fps) as usize;
        if frame < self.blinks[clip_i].len {
            Some((clip_i, frame))
        } else {
            None
        }
    }

    /// Si `blink` está en curso, desvía este track al párpado o lo tapa.
    /// `None` significa seguir el frame normal del estado.
    pub fn overlay<F>(
        &self,
        track: usize,
        blink: Option<(usize, usize)>,
        mut active: F,
    ) -> Option<Overlay>
    where
        F: FnMut(usize, usize) -> bool,
    {
        let (clip_i, frame) = blink?;
        let clip = self.blinks.get(clip_i)?;
        let src = clip.start + frame;
        if clip.tracks.contains(&track) {
            return Some(Overlay::Sample(src));
        }
        let showing = clip.tracks.iter().any(|&ti| active(ti, src));
        if showing && clip.hide.contains(&track) {
            Some(Overlay::Hidden)
        } else {
            None
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

    let mut data = ReanimData {
        fps,
        tracks,
        animations,
        states,
    };
    // Los `anim_blink` del archivo no son un estado: el párpado va encima del reposo.
    fold_blinks(&mut data);
    Ok(data)
}

fn is_blink_name(name: &str) -> bool {
    name.to_ascii_lowercase().contains("blink")
}

/// Ojo, ceja, boca o mejilla: no cuentan como el cuerpo de la planta.
fn is_detail(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("blink")
        || n.contains("eye")
        || n.contains("brow")
        || n.contains("cheek")
        || n.contains("mouth")
        || n.contains("lip")
}

fn is_eye_track(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("eye") && !n.contains("brow") && !n.contains("blink")
}

/// Reposo, o la papa ya armada: ahí es donde el personaje se queda quieto.
fn is_rest(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    !is_blink_name(name) && (n.contains("idle") || n.contains("armed") || n == "full")
}

fn frame_on(track: &Track, frame: usize) -> bool {
    track
        .frames
        .get(frame)
        .and_then(|r| r.as_ref())
        .is_some_and(|r| r.active && r.image.is_some())
}

fn real_body_count(data: &ReanimData, frame: usize) -> usize {
    data.tracks
        .iter()
        .filter(|t| t.has_image && !is_detail(&t.name) && frame_on(t, frame))
        .count()
}

fn blink_sprite_on(data: &ReanimData, frame: usize) -> bool {
    data.tracks
        .iter()
        .any(|t| t.has_image && is_blink_name(&t.name) && frame_on(t, frame))
}

/// El rango es un parpadeo suelto (el cuerpo está apagado) y no una animación entera.
fn is_floating(data: &ReanimData, start: usize, end: usize) -> bool {
    let blink_frames: Vec<usize> = (start..=end)
        .filter(|&f| blink_sprite_on(data, f))
        .collect();
    let frames: Vec<usize> = if blink_frames.is_empty() {
        (start..=end).collect()
    } else {
        blink_frames
    };
    !frames.is_empty() && frames.iter().all(|&f| real_body_count(data, f) == 0)
}

fn overlaps_other(data: &ReanimData, blink: &AnimRange) -> bool {
    let len = blink.end - blink.start + 1;
    data.animations.iter().any(|other| {
        if is_blink_name(&other.name) {
            return false;
        }
        let start = blink.start.max(other.start);
        let end = blink.end.min(other.end);
        if end < start {
            return false;
        }
        (end - start + 1) * 2 > len
    })
}

fn active_span(
    data: &ReanimData,
    tracks: &[usize],
    start: usize,
    end: usize,
) -> Option<(usize, usize)> {
    let mut first = None;
    let mut last = 0;
    for frame in start..=end {
        if tracks.iter().any(|&ti| frame_on(&data.tracks[ti], frame)) {
            if first.is_none() {
                first = Some(frame);
            }
            last = frame;
        }
    }
    first.map(|s| (s, last))
}

/// Une rachas del mismo track separadas por uno o dos frames apagados
/// (el ojo abierto entre un parpadeo doble).
fn merge_runs(frames: &[usize]) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut iter = frames.iter().copied();
    let Some(mut start) = iter.next() else {
        return runs;
    };
    let mut end = start;
    for frame in iter {
        if frame - end <= 3 {
            end = frame;
        } else {
            runs.push((start, end));
            start = frame;
            end = frame;
        }
    }
    runs.push((start, end));
    runs
}

fn host_frames(state: &AnimState) -> Vec<usize> {
    let mut frames = Vec::new();
    let push = |frames: &mut Vec<usize>, start: usize, n: usize| {
        for k in 0..n {
            frames.push(start + k);
        }
    };
    push(
        &mut frames,
        state.body_start,
        state.body_cycle.unwrap_or(state.len).max(1),
    );
    push(
        &mut frames,
        state.head_start,
        state.head_cycle.unwrap_or(state.len).max(1),
    );
    for group in &state.groups {
        push(
            &mut frames,
            group.start,
            group.cycle.unwrap_or(state.len).max(1),
        );
    }
    frames
}

fn state_for_marker(states: &[AnimState], marker: &str) -> Option<usize> {
    if let Some(i) = states.iter().position(|s| s.name == marker) {
        return Some(i);
    }
    let m = marker.to_ascii_lowercase();
    let find = |pred: fn(&str) -> bool| -> Option<usize> {
        states
            .iter()
            .position(|s| pred(&s.name.to_ascii_lowercase()))
    };
    if m.contains("idlehigh") {
        return find(|n| n.contains("idlehigh"));
    }
    if m.contains("unarmed") {
        return find(|n| n.contains("unarmed"));
    }
    if m.contains("armed") {
        return find(|n| n.contains("armed") && !n.contains("unarmed"));
    }
    if m.contains("idle") {
        for name in ["idle", "anim_idle", "anim_full_idle", "full"] {
            if let Some(i) = states.iter().position(|s| s.name == name) {
                return Some(i);
            }
        }
    }
    None
}

fn pick_host_name(anims: &[AnimRange], clip_start: usize, clip_end: usize) -> Option<String> {
    let rests: Vec<&AnimRange> = anims.iter().filter(|a| is_rest(&a.name)).collect();
    if let Some(next) = rests
        .iter()
        .filter(|a| a.start > clip_end)
        .min_by_key(|a| a.start)
    {
        let gap = next.start - clip_end - 1;
        if gap <= 8 {
            return Some(next.name.clone());
        }
    }
    if let Some(prev) = rests
        .iter()
        .filter(|a| a.end < clip_start)
        .max_by_key(|a| a.end)
    {
        let gap = clip_start - prev.end - 1;
        if gap <= 3 {
            return Some(prev.name.clone());
        }
    }
    rests
        .iter()
        .min_by_key(|a| {
            if clip_end < a.start {
                a.start - clip_end
            } else if a.end < clip_start {
                clip_start - a.end
            } else {
                0
            }
        })
        .map(|a| a.name.clone())
}

fn host_index(data: &ReanimData, clip_start: usize, clip_end: usize) -> Option<usize> {
    if let Some(name) = pick_host_name(&data.animations, clip_start, clip_end) {
        if let Some(i) = state_for_marker(&data.states, &name) {
            return Some(i);
        }
    }
    let rests: Vec<usize> = data
        .states
        .iter()
        .enumerate()
        .filter(|(_, s)| is_rest(&s.name))
        .map(|(i, _)| i)
        .collect();
    if rests.len() == 1 {
        return Some(rests[0]);
    }
    if data.states.len() == 1 {
        return Some(0);
    }
    data.states
        .iter()
        .position(|s| s.name == "full" || s.name == "idle" || s.name == "anim_idle")
}

struct RawClip {
    start: usize,
    end: usize,
    tracks: Vec<usize>,
}

fn push_clip(clips: &mut Vec<RawClip>, start: usize, end: usize, tracks: Vec<usize>) {
    if tracks.is_empty() || end < start {
        return;
    }
    if let Some(existing) = clips.iter_mut().find(|c| c.start == start && c.end == end) {
        for track in tracks {
            if !existing.tracks.contains(&track) {
                existing.tracks.push(track);
            }
        }
        return;
    }
    clips.push(RawClip { start, end, tracks });
}

/// Quita el parpadeo de la lista de estados y lo cuelga del reposo.
///
/// En el `.reanim` el parpadeo suele ser un bloque donde el cuerpo está apagado
/// y solo se ve el párpado. Reproducirlo tal cual deja los ojos flotando. Aquí
/// el cuerpo sigue el reposo y el párpado se dibuja encima, de vez en cuando.
pub fn fold_blinks(data: &mut ReanimData) {
    for state in &mut data.states {
        state.blinks.clear();
    }

    let blink_markers: Vec<AnimRange> = data
        .animations
        .iter()
        .filter(|a| is_blink_name(&a.name))
        .cloned()
        .collect();

    let mut drop_names = Vec::new();
    let mut clips: Vec<RawClip> = Vec::new();

    for marker in &blink_markers {
        let floating = is_floating(data, marker.start, marker.end);
        let overlap = overlaps_other(data, marker);
        if floating || overlap {
            drop_names.push(marker.name.clone());
        }
        if !floating || overlap {
            continue;
        }
        let host_name = pick_host_name(&data.animations, marker.start, marker.end);
        let host = host_name
            .as_deref()
            .and_then(|name| data.animations.iter().find(|a| a.name == name));
        let show: Vec<usize> = data
            .tracks
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                t.has_image
                    && (marker.start..=marker.end).any(|f| frame_on(t, f))
                    && host.is_none_or(|h| !(h.start..=h.end).any(|f| frame_on(t, f)))
            })
            .map(|(i, _)| i)
            .collect();
        if let Some((start, end)) = active_span(data, &show, marker.start, marker.end) {
            push_clip(&mut clips, start, end, show);
        }
    }

    data.states
        .retain(|s| !drop_names.iter().any(|n| n == &s.name));

    for (ti, track) in data.tracks.iter().enumerate() {
        if !track.has_image || !is_blink_name(&track.name) {
            continue;
        }
        let active: Vec<usize> = (0..track.frames.len())
            .filter(|&f| frame_on(track, f))
            .collect();
        for (start, end) in merge_runs(&active) {
            let inside_marker = blink_markers
                .iter()
                .any(|m| start <= m.end && m.start <= end);
            if inside_marker || (start..=end).any(|f| real_body_count(data, f) > 0) {
                continue;
            }
            push_clip(&mut clips, start, end, vec![ti]);
        }
    }

    if clips.is_empty() && drop_names.is_empty() {
        return;
    }

    // El archivo entero (`full`) mete el parpadeo en el bucle y la planta desaparece.
    // El reposo se queda solo con los frames en los que el cuerpo está encendido.
    let trimmed: Vec<(usize, usize, usize)> = data
        .states
        .iter()
        .enumerate()
        .filter(|(_, state)| state.name == "full" && is_plain(state))
        .filter_map(|(i, state)| {
            let mut start = state.body_start;
            let mut end = state.body_start + state.len.saturating_sub(1);
            while start <= end && real_body_count(data, start) == 0 {
                start += 1;
            }
            while end > start && real_body_count(data, end) == 0 {
                end -= 1;
            }
            if start <= end && (start != state.body_start || end + 1 - start != state.len) {
                Some((i, start, end + 1 - start))
            } else {
                None
            }
        })
        .collect();
    for (i, start, len) in trimmed {
        data.states[i].body_start = start;
        data.states[i].head_start = start;
        data.states[i].len = len;
    }

    clips.sort_by_key(|c| c.start);
    for clip in clips {
        let Some(index) = host_index(data, clip.start, clip.end) else {
            continue;
        };
        let frames = host_frames(&data.states[index]);
        let hide: Vec<usize> = data
            .tracks
            .iter()
            .enumerate()
            .filter(|(ti, t)| {
                !clip.tracks.contains(ti)
                    && is_eye_track(&t.name)
                    && frames.iter().any(|&f| frame_on(t, f))
                    && !(clip.start..=clip.end).any(|f| frame_on(t, f))
            })
            .map(|(i, _)| i)
            .collect();
        data.states[index].blinks.push(BlinkClip {
            start: clip.start,
            len: clip.end - clip.start + 1,
            tracks: clip.tracks,
            hide,
        });
    }
}

fn is_plain(state: &AnimState) -> bool {
    state.body_start == state.head_start
        && state.body_cycle.is_none()
        && state.head_cycle.is_none()
        && state.groups.is_empty()
        && state.head_dx.is_empty()
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

    fn track_named(data: &ReanimData, name: &str) -> usize {
        data.tracks.iter().position(|t| t.name == name).expect(name)
    }

    fn on(data: &ReanimData) -> impl Fn(usize, usize) -> bool + '_ {
        |ti, frame| {
            data.tracks
                .get(ti)
                .and_then(|t| t.frames.get(frame))
                .and_then(|r| r.as_ref())
                .is_some_and(|r| r.active)
        }
    }

    #[test]
    fn el_girasol_parpadea_encima_del_reposo() {
        let data = parse_reanim(&asset("animations/SunFlower/SunFlower.reanim")).unwrap();
        assert!(data
            .states
            .iter()
            .all(|s| !s.name.to_lowercase().contains("blink")));
        assert_eq!(data.states.len(), 1);
        let full = &data.states[0];
        assert_eq!(full.name, "full");
        // Frames 0..3 son solo el párpado; el reposo empieza cuando aparece la flor.
        assert_eq!(full.body_start, 4);
        assert_eq!(full.len, 25);
        assert_eq!(full.blinks.len(), 1);
        let clip = &full.blinks[0];
        assert_eq!((clip.start, clip.len), (1, 3));
        let blink = track_named(&data, "anim_blink");
        let head = track_named(&data, "anim_idle");
        let stalk = track_named(&data, "stalk_top");
        assert!(clip.tracks.contains(&blink));
        assert!(!clip.hide.contains(&head));
        let hit = Some((0, 1));
        assert_eq!(
            full.overlay(blink, hit, on(&data)),
            Some(Overlay::Sample(2))
        );
        assert_eq!(full.overlay(head, hit, on(&data)), None);
        assert_eq!(full.overlay(stalk, hit, on(&data)), None);
        assert!(full.sample(PartGroup::Body, 0).0 >= 4);
    }

    #[test]
    fn el_parpadeo_no_es_un_estado_y_respeta_el_reposo() {
        let data = parse_reanim(&asset("animations/Cabbagepult/Cabbagepult.reanim")).unwrap();
        assert!(data
            .states
            .iter()
            .all(|s| !s.name.to_lowercase().contains("blink")));
        let idle = data.states.iter().find(|s| s.name == "anim_idle").unwrap();
        assert_eq!(idle.body_start, 5);
        assert_eq!(idle.len, 31);
        assert_eq!(idle.blinks.len(), 1);
        let shoot = data
            .states
            .iter()
            .find(|s| s.name == "anim_shooting")
            .unwrap();
        assert!(
            shoot.blinks.is_empty(),
            "el disparo ya trae su parpadeo en la línea de tiempo"
        );
        let blink = track_named(&data, "Cabbagepult_blink");
        let stalk = track_named(&data, "Cabbagepult_stalk1");
        assert!(idle.blinks[0].tracks.contains(&blink));
        assert_eq!(
            idle.overlay(stalk, Some((0, 0)), on(&data)),
            None,
            "el tallo sigue en el frame del reposo"
        );
    }

    #[test]
    fn la_nuez_tiene_tres_parpadeos_sobre_el_mismo_reposo() {
        let data = parse_reanim(&asset("animations/Wallnut/Wallnut.reanim")).unwrap();
        let idle = data.states.iter().find(|s| s.name == "anim_idle").unwrap();
        assert_eq!(idle.body_start, 0);
        assert_eq!(idle.len, 17);
        assert_eq!(idle.blinks.len(), 3);
        let face = track_named(&data, "anim_face");
        for clip in &idle.blinks {
            assert!(!clip.tracks.contains(&face));
            assert!(!clip.hide.contains(&face));
        }
        // twice: frames 23..29 con un frame de ojos abiertos en medio.
        let twice = idle.blinks.iter().find(|c| c.start == 23).unwrap();
        assert_eq!(twice.len, 7);
    }

    #[test]
    fn la_papa_armada_parpadea_y_tapa_el_ojo_abierto() {
        let data = parse_reanim(&asset("animations/PotatoMine/PotatoMine.reanim")).unwrap();
        assert!(data.states.iter().all(|s| s.name != "anim_blink"));
        let idle = data.states.iter().find(|s| s.name == "anim_idle").unwrap();
        let armed = data.states.iter().find(|s| s.name == "anim_armed").unwrap();
        assert!(idle.blinks.is_empty());
        assert_eq!(armed.blinks.len(), 1);
        let eye = track_named(&data, "anim_eye");
        let blink = track_named(&data, "PotatoMine_blink");
        assert!(armed.blinks[0].hide.contains(&eye));
        assert!(armed.blinks[0].tracks.contains(&blink));
        assert_eq!(
            armed.overlay(eye, Some((0, 0)), on(&data)),
            Some(Overlay::Hidden)
        );
        let rock = track_named(&data, "PotatoMine_rock1");
        assert_eq!(armed.overlay(rock, Some((0, 0)), on(&data)), None);
    }

    #[test]
    fn el_carbonizado_no_duplica_el_parpadeo_que_ya_trae_crumble() {
        let data = parse_reanim(&asset("animations/Zombie/Zombie_charred.reanim")).unwrap();
        assert!(data.states.iter().all(|s| s.name != "anim_blink"));
        let crumble = data
            .states
            .iter()
            .find(|s| s.name == "anim_crumble")
            .unwrap();
        assert!(crumble.blinks.is_empty());
        assert_eq!(crumble.body_start, 0);
    }

    #[test]
    fn otros_reposos_tambien_reciben_el_parpadeo() {
        let expect = |rel: &str, state: &str, n: usize, body: usize, len: usize| {
            let data = parse_reanim(&asset(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
            assert!(
                data.states
                    .iter()
                    .all(|s| !s.name.to_lowercase().contains("blink")),
                "{rel}: {:?}",
                data.states
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
            );
            let host = data
                .states
                .iter()
                .find(|s| s.name == state)
                .unwrap_or_else(|| {
                    panic!(
                        "{rel} no tiene {state}: {:?}",
                        data.states
                            .iter()
                            .map(|s| s.name.as_str())
                            .collect::<Vec<_>>()
                    )
                });
            assert_eq!(host.blinks.len(), n, "{rel} {state}");
            assert_eq!((host.body_start, host.len), (body, len), "{rel} {state}");
        };
        expect("animations/Tallnut/Tallnut.reanim", "full", 2, 20, 17);
        expect("animations/Garlic/Garlic.reanim", "anim_idle", 1, 4, 15);
        expect("animations/LilyPad/LilyPad.reanim", "full", 1, 0, 1);
        expect("animations/Caltrop/Caltrop.reanim", "anim_idle", 1, 4, 21);
        expect(
            "animations/TwinSunflower/TwinSunflower.reanim",
            "anim_idle",
            2,
            9,
            25,
        );
        expect(
            "animations/Torchwood/Torchwood.reanim",
            "anim_idle",
            1,
            4,
            21,
        );
    }

    #[test]
    fn el_parpadeo_espera_y_no_ocurre_en_cada_vuelta() {
        let data = parse_reanim(&asset("animations/SunFlower/SunFlower.reanim")).unwrap();
        let full = &data.states[0];
        let fps = data.fps;
        assert!(full.blink_at(0.0, fps, 0.0).is_none());
        let dur = full.blinks[0].len as f32 / fps;
        let begin = BLINK_PERIOD - dur;
        assert_eq!(full.blink_at(begin, fps, 0.0), Some((0, 0)));
        assert_eq!(full.blink_at(begin + 1.0 / fps, fps, 0.0), Some((0, 1)));
        assert!(full.blink_at(begin - 0.05, fps, 0.0).is_none());
    }

    #[test]
    fn ningun_parpadeo_suelto_queda_como_estado() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/animations");
        let mut files = 0;
        for entry in walk_reanims(&root) {
            files += 1;
            let data = parse_reanim(&entry).unwrap_or_else(|e| panic!("{}: {e}", entry.display()));
            for state in &data.states {
                if !state.name.to_lowercase().contains("blink") {
                    continue;
                }
                let start = state.body_start;
                let end = start + state.len.saturating_sub(1);
                let body = (start..=end).any(|f| {
                    data.tracks.iter().any(|t| {
                        t.has_image
                            && !t.name.to_ascii_lowercase().contains("blink")
                            && !t.name.to_ascii_lowercase().contains("eye")
                            && t.frames
                                .get(f)
                                .and_then(|r| r.as_ref())
                                .is_some_and(|r| r.active && r.image.is_some())
                    })
                });
                assert!(
                    body,
                    "{} conserva {} pero el cuerpo está apagado",
                    entry.display(),
                    state.name
                );
            }
        }
        assert!(files > 100, "se recorrieron {files} reanim");
    }

    fn walk_reanims(dir: &Path) -> Vec<std::path::PathBuf> {
        let mut out = Vec::new();
        let Ok(read) = std::fs::read_dir(dir) else {
            return out;
        };
        for entry in read.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(walk_reanims(&path));
            } else if path.extension().is_some_and(|e| e == "reanim") {
                out.push(path);
            }
        }
        out
    }
}
