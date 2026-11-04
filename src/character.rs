//! Código compartido por el visor y la partida: crear las piezas de un
//! personaje a partir de su `.reanim` y colocarlas según el frame actual.

use crate::components::CharacterPart;
use crate::reanim::{AnimState, Overlay, ReanimData};
use crate::resources::CharacterLibraries;
use crate::utils::{asset_exists, entity_name, image_to_path};
use crate::zombie_style::PlainStyle;
use crate::Character;
use bevy::math::{Affine3A, Vec3A};
use bevy::prelude::*;
use bevy::render::view::RenderLayers;

/// Reproductor de animación de un personaje (va en la entidad raíz).
/// El visor lleva su propio estado en `ViewerState`; la partida usa este.
#[derive(Component, Debug, Clone)]
pub struct Animator {
    pub character: Character,
    /// Índice dentro de `ReanimData::states`.
    pub state: usize,
    /// Tiempo acumulado del estado actual, ya multiplicado por `speed`.
    pub time: f32,
    /// Multiplicador de velocidad (1.0 = la del `.reanim`).
    pub speed: f32,
    pub looping: bool,
    pub paused: bool,
}

impl Animator {
    pub fn new(character: Character, state: usize) -> Self {
        Self {
            character,
            state,
            time: 0.0,
            speed: 1.0,
            looping: true,
            paused: false,
        }
    }

    /// Cambia de estado. Si ya estaba en ese estado no reinicia la animación.
    pub fn play(&mut self, state: usize, looping: bool, speed: f32) {
        if self.state != state || self.looping != looping {
            self.state = state;
            self.looping = looping;
            self.time = 0.0;
        }
        self.speed = speed;
    }

    /// Frame dentro del bucle del estado actual (0..len).
    pub fn local_frame(&self, data: &ReanimData) -> usize {
        let Some(state) = data.states.get(self.state) else {
            return 0;
        };
        let len = state.len.max(1);
        let frame = (self.time * data.fps) as usize;
        if self.looping {
            frame % len
        } else {
            frame.min(len - 1)
        }
    }

    /// Una animación que no se repite ya llegó a su último frame.
    pub fn finished(&self, data: &ReanimData) -> bool {
        match data.states.get(self.state) {
            Some(state) => !self.looping && (self.time * data.fps) as usize >= state.len.max(1),
            None => true,
        }
    }
}

/// Crea la entidad raíz de un personaje con todas sus piezas como hijas.
/// El que llama añade sus propios componentes a la raíz devuelta.
pub fn spawn_character(
    commands: &mut Commands,
    asset_server: &AssetServer,
    data: &ReanimData,
    character: Character,
    transform: Transform,
    initial_state: usize,
) -> Entity {
    let root = commands
        .spawn((
            transform,
            Visibility::default(),
            Animator::new(character, initial_state),
        ))
        .id();
    spawn_parts(
        commands,
        asset_server,
        data,
        entity_name(character),
        root,
        false,
        None,
    );
    root
}

/// Resultado de crear las piezas de un personaje.
pub struct SpawnReport {
    pub spawned: usize,
    pub skipped: usize,
    /// Ids que no se pudieron resolver, como mucho ocho, para el HUD.
    pub missing: Vec<String>,
    /// La mano donde se engancha la bandera del zombi normal, si existe.
    pub flaghand: Option<Entity>,
}

/// Crea los sprites de las piezas visibles del personaje como hijos de `root`.
///
/// `layer`, si viene, pone cada sprite en esa capa de render. Sin ella se quedan
/// en la capa 0, la de la cámara del césped.
pub fn spawn_parts(
    commands: &mut Commands,
    asset_server: &AssetServer,
    data: &ReanimData,
    entity: &str,
    root: Entity,
    verbose: bool,
    layer: Option<usize>,
) -> SpawnReport {
    let mut count = 0;
    let mut skipped = 0;
    let mut missing = Vec::new();
    let mut flaghand = None;

    let note_missing = |missing: &mut Vec<String>, image: &str, track: &str| {
        if missing.len() < 8 {
            missing.push(format!("{image} ({track})"));
        }
    };

    for (idx, track) in data.tracks.iter().enumerate() {
        if !track.has_image {
            continue;
        }

        let first_image = match track
            .frames
            .iter()
            .find_map(|r| r.as_ref().and_then(|r| r.image.clone()))
        {
            Some(i) => i,
            None => {
                if verbose {
                    println!("⚠️  Track '{}' sin imagen en ningún frame", track.name);
                }
                skipped += 1;
                continue;
            }
        };

        let rel = match image_to_path(&first_image, entity) {
            Some(p) => p,
            None => {
                if verbose {
                    println!("⚠️  Sin mapeo para: {} (track {})", first_image, track.name);
                }
                note_missing(&mut missing, &first_image, &track.name);
                skipped += 1;
                continue;
            }
        };

        if !asset_exists(&rel) {
            if verbose {
                println!("⚠️  Falta asset: {} (track {})", rel, track.name);
            }
            note_missing(&mut missing, &first_image, &track.name);
            skipped += 1;
            continue;
        }

        let z = idx as f32 * 0.001;
        let track_name = track.name.clone();
        commands.entity(root).with_children(|parent| {
            let mut child = parent.spawn((
                Sprite {
                    image: asset_server.load(&rel),
                    anchor: bevy::sprite::Anchor::TopLeft,
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, z),
                Visibility::default(),
                CharacterPart {
                    track: idx,
                    current_image: Some(first_image.clone()),
                    axes: None,
                },
            ));
            if let Some(layer) = layer {
                child.insert(RenderLayers::layer(layer));
            }
            let id = child.id();
            if track_name == "Zombie_flaghand" {
                flaghand = Some(id);
            }
        });
        count += 1;
    }

    SpawnReport {
        spawned: count,
        skipped,
        missing,
        flaghand,
    }
}

/// Dibuja cada pieza con sus ejes reales, sesgo incluido.
///
/// `Transform` solo guarda traslación, rotación y escala, así que un sesgo
/// (`kx != ky`, p. ej. el cactus inclinándose al disparar) se perdía y el cuerpo
/// se despegaba de la cabeza. El sprite se dibuja con `GlobalTransform`, que sí
/// admite cualquier matriz afín: se recalcula aquí, tras la propagación de Bevy.
pub fn apply_part_shear(
    mut parts: Query<(&Parent, &Transform, &CharacterPart, &mut GlobalTransform)>,
    parents: Query<&GlobalTransform, Without<CharacterPart>>,
) {
    for (parent, transform, part, mut global) in &mut parts {
        let (Some(axes), Ok(parent_global)) = (part.axes, parents.get(parent.get())) else {
            continue;
        };
        let local = Affine3A::from_cols(
            Vec3A::new(axes.x_axis.x, axes.x_axis.y, 0.0),
            Vec3A::new(axes.y_axis.x, axes.y_axis.y, 0.0),
            Vec3A::Z,
            Vec3A::from(transform.translation),
        );
        *global = GlobalTransform::from(parent_global.affine() * local);
    }
}

/// Coloca una pieza según el frame `local` (0..len) del estado `anim`.
/// El cuerpo y la cabeza leen frames de sitios distintos (ver `AnimState`).
/// `blink` es el parpadeo en curso, si toca: el cuerpo sigue en el reposo y
/// el párpado se dibuja encima.
pub fn pose_part(
    data: &ReanimData,
    anim: &AnimState,
    local: usize,
    entity: &str,
    style: Option<PlainStyle>,
    asset_server: &AssetServer,
    xf: &mut Transform,
    sprite: &mut Sprite,
    vis: &mut Visibility,
    part: &mut CharacterPart,
    blink: Option<(usize, usize)>,
) {
    let track = &data.tracks[part.track];
    // El zombi normal lleva cono, cubo, puerta y demás en el mismo archivo.
    // El estilo decide qué grupo se ve; el frame no basta.
    if let Some(style) = style {
        if !style.allows(&track.name) {
            *vis = Visibility::Hidden;
            return;
        }
    }
    let is_prop = track.name.starts_with("anim_");
    let on = |ti: usize, frame: usize| {
        data.tracks
            .get(ti)
            .and_then(|t| t.frames.get(frame))
            .and_then(|r| r.as_ref())
            .is_some_and(|r| r.active)
    };
    let (frame, dx, adjust) = match anim.overlay(part.track, blink, on) {
        Some(Overlay::Hidden) => {
            *vis = Visibility::Hidden;
            return;
        }
        Some(Overlay::Sample(frame)) => {
            let adjust = blink.and_then(|hit| anim.blink_follow(data, hit, local));
            if adjust.is_some_and(|a| !a.visible) {
                *vis = Visibility::Hidden;
                return;
            }
            (frame, 0.0, adjust)
        }
        None => {
            let (frame, dx) = anim.sample(track.group, local);
            (frame, dx, None)
        }
    };

    match track.frames.get(frame).and_then(|r| r.as_ref()) {
        Some(r) => {
            if !r.active {
                *vis = Visibility::Hidden;
                return;
            }
            let mut r = r.clone();
            if let Some(adjust) = adjust {
                adjust.apply(&mut r);
            }
            *vis = Visibility::Inherited;
            xf.translation = Vec3::new(r.x + dx, -r.y, xf.translation.z);
            xf.scale = Vec3::new(r.sx, r.sy, 1.0);
            xf.rotation = Quat::from_rotation_z(-r.kx.to_radians());
            // Lo anterior solo sirve si `kx == ky`; con sesgo (el cactus al
            // disparar) manda `axes`.
            let (ax, ay) = r.axes();
            part.axes = Some(Mat2::from_cols(Vec2::from(ax), Vec2::from(ay)));

            // Cambiar imagen si el track usa múltiples sprites (ej: blink1/blink2)
            if let Some(ref new_img) = r.image {
                if part.current_image.as_deref() != Some(new_img.as_str()) {
                    // Se marca como procesada aunque falte el archivo, para no
                    // volver a consultar el disco en cada frame.
                    part.current_image = Some(new_img.clone());
                    if let Some(rel) = image_to_path(new_img, entity) {
                        if asset_exists(&rel) {
                            sprite.image = asset_server.load(&rel);
                        }
                    }
                }
            }
        }
        None => {
            if is_prop {
                *vis = Visibility::Hidden;
            }
        }
    }
}

/// Avanza y dibuja todos los personajes con `Animator` (la usa la partida).
pub fn animate_characters(
    time: Res<Time>,
    libraries: Res<CharacterLibraries>,
    asset_server: Res<AssetServer>,
    mut roots: Query<(&mut Animator, &Children)>,
    mut parts: Query<(
        &mut Transform,
        &mut Sprite,
        &mut Visibility,
        &mut CharacterPart,
    )>,
) {
    for (mut animator, children) in &mut roots {
        let data = libraries.data(animator.character);
        if data.states.is_empty() {
            continue;
        }
        if !animator.paused {
            animator.time += time.delta_secs() * animator.speed;
        }

        let index = animator.state % data.states.len();
        let local = animator.local_frame(data);
        let state = &data.states[index];
        let blink = state.blink_at(animator.time, data.fps, 0.0);
        let entity = entity_name(animator.character);
        // La partida, cuando dibuje, usa el zombi normal sin accesorios mezclados.
        let style = match animator.character {
            Character::Zombie => Some(PlainStyle::Normal),
            Character::PeaShooter => None,
        };

        let mut iter = parts.iter_many_mut(children);
        while let Some((mut xf, mut sprite, mut vis, mut part)) = iter.fetch_next() {
            pose_part(
                data,
                state,
                local,
                entity,
                style,
                &asset_server,
                &mut xf,
                &mut sprite,
                &mut vis,
                &mut part,
                blink,
            );
        }
    }
}
