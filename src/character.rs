//! Código compartido por el visor y la partida: crear las piezas de un
//! personaje a partir de su `.reanim` y colocarlas según el frame actual.

use crate::components::CharacterPart;
use crate::reanim::{AnimState, PartGroup, ReanimData};
use crate::resources::CharacterLibraries;
use crate::utils::{asset_exists, get_allowlist, image_to_path};
use crate::Character;
use bevy::prelude::*;

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
    spawn_parts(commands, asset_server, data, character, root, false);
    root
}

/// Crea los sprites de las piezas visibles del personaje como hijos de `root`.
/// Devuelve `(piezas creadas, piezas saltadas)`.
pub fn spawn_parts(
    commands: &mut Commands,
    asset_server: &AssetServer,
    data: &ReanimData,
    character: Character,
    root: Entity,
    verbose: bool,
) -> (usize, usize) {
    let allowlist = get_allowlist(character);
    let mut count = 0;
    let mut skipped = 0;

    for (idx, track) in data.tracks.iter().enumerate() {
        if !allowlist.contains(&track.name.as_str()) {
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

        let rel = match image_to_path(&first_image, character) {
            Some(p) => p,
            None => {
                if verbose {
                    println!("⚠️  Sin mapeo para: {} (track {})", first_image, track.name);
                }
                skipped += 1;
                continue;
            }
        };

        if !asset_exists(&rel) {
            if verbose {
                println!("⚠️  Falta asset: {} (track {})", rel, track.name);
            }
            skipped += 1;
            continue;
        }

        let z = idx as f32 * 0.001;
        commands.entity(root).with_children(|parent| {
            parent.spawn((
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
                },
            ));
        });
        count += 1;
    }

    (count, skipped)
}

/// Coloca una pieza según el frame `local` (0..len) del estado `anim`.
/// El cuerpo y la cabeza leen frames de sitios distintos (ver `AnimState`).
pub fn pose_part(
    data: &ReanimData,
    anim: &AnimState,
    local: usize,
    character: Character,
    asset_server: &AssetServer,
    xf: &mut Transform,
    sprite: &mut Sprite,
    vis: &mut Visibility,
    part: &mut CharacterPart,
) {
    let track = &data.tracks[part.track];
    let is_prop = track.name.starts_with("anim_");
    let is_head = track.group == PartGroup::Head;
    let frame = local + if is_head { anim.head_start } else { anim.body_start };
    let dx = if is_head {
        anim.head_dx.get(local).copied().unwrap_or(0.0)
    } else {
        0.0
    };

    match track.frames.get(frame).and_then(|r| r.as_ref()) {
        Some(r) => {
            if !r.active {
                *vis = Visibility::Hidden;
                return;
            }
            *vis = Visibility::Inherited;
            xf.translation = Vec3::new(r.x + dx, -r.y, xf.translation.z);
            xf.scale = Vec3::new(r.sx, r.sy, 1.0);
            xf.rotation = Quat::from_rotation_z(-r.kx.to_radians());

            // Cambiar imagen si el track usa múltiples sprites (ej: blink1/blink2)
            if let Some(ref new_img) = r.image {
                if part.current_image.as_deref() != Some(new_img.as_str()) {
                    // Se marca como procesada aunque falte el archivo, para no
                    // volver a consultar el disco en cada frame.
                    part.current_image = Some(new_img.clone());
                    if let Some(rel) = image_to_path(new_img, character) {
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
        let character = animator.character;

        let mut iter = parts.iter_many_mut(children);
        while let Some((mut xf, mut sprite, mut vis, mut part)) = iter.fetch_next() {
            pose_part(
                data,
                state,
                local,
                character,
                &asset_server,
                &mut xf,
                &mut sprite,
                &mut vis,
                &mut part,
            );
        }
    }
}
